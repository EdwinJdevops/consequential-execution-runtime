use cer_core::{ActionArguments, ApprovalBinding, ProposedAction};
use cer_postgres_adapter::transactional::{
    execute_atomic, inspect_durable_outcome, AtomicOutcome, DurableOutcome,
};
use postgres::{Client, NoTls};
use std::sync::{Arc, Barrier};

fn setup(db: &mut Client) {
    db.batch_execute("SELECT pg_advisory_lock(72451639);")
        .unwrap();
    db.batch_execute(
        "CREATE TABLE IF NOT EXISTS cer_resources (
            resource_id TEXT PRIMARY KEY, version BIGINT NOT NULL, payload JSONB NOT NULL
        );
        ALTER TABLE cer_resources ADD COLUMN IF NOT EXISTS last_operation_id TEXT;",
    )
    .unwrap();
    db.batch_execute(cer_postgres_adapter::journal::SCHEMA)
        .unwrap();
    db.batch_execute(include_str!(
        "../migrations/0004_atomic_effect_evidence.sql"
    ))
    .unwrap();
    db.batch_execute("SELECT pg_advisory_unlock(72451639);")
        .unwrap();
}

fn action(id: &str, resource: &str) -> ProposedAction {
    ProposedAction {
        action_id: id.into(),
        resource_id: resource.into(),
        operation: "update".into(),
        arguments: ActionArguments::from_json_str(r#"{"value":1}"#).unwrap(),
        observed_state: "3".into(),
        policy_version: "p1".into(),
    }
}

#[test]
fn rollback_before_commit_leaves_neither_journal_nor_effect() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("rollback-{}", std::process::id());
    let mut db = Client::connect(&url, NoTls).unwrap();
    setup(&mut db);
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
        &[&id],
    )
    .unwrap();
    {
        let mut tx = db.transaction().unwrap();
        tx.execute(
            "INSERT INTO cer_operation_journal (operation_id, fingerprint, status)
             VALUES ($1, 'fingerprint', 'SUCCEEDED')",
            &[&id],
        )
        .unwrap();
        tx.execute(
            "UPDATE cer_resources SET version = 4, last_operation_id = $1
             WHERE resource_id = $1",
            &[&id],
        )
        .unwrap();
        // Transaction dropped without commit, as after an interrupted worker.
    }
    let journal: i64 = db
        .query_one(
            "SELECT count(*) FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(journal, 0);
    let version: i64 = db
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version, 3);
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn reconnect_after_commit_reads_durable_outcome() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("uncertain-{}", std::process::id());
    let a = action(&id, &id);
    let approval = ApprovalBinding::bind(&a).unwrap();
    {
        let mut db = Client::connect(&url, NoTls).unwrap();
        setup(&mut db);
        db.execute(
            "INSERT INTO cer_resources (resource_id, version, payload)
             VALUES ($1, 3, '{}'::jsonb)",
            &[&id],
        )
        .unwrap();
        assert_eq!(
            execute_atomic(&mut db, &approval, &a, "3").unwrap(),
            AtomicOutcome::Applied
        );
        // Simulate a caller losing the successful response.
    }
    let mut recovered = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        inspect_durable_outcome(&mut recovered, &a).unwrap(),
        DurableOutcome::Succeeded
    );
    assert_eq!(
        execute_atomic(&mut recovered, &approval, &a, "3").unwrap(),
        AtomicOutcome::AlreadyApplied
    );
    recovered
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    recovered
        .execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn simultaneous_workers_mutate_once() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("workers-{}", std::process::id());
    let mut db = Client::connect(&url, NoTls).unwrap();
    setup(&mut db);
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
        &[&id],
    )
    .unwrap();
    drop(db);
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let url = url.clone();
            let id = id.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let a = action(&id, &id);
                let approval = ApprovalBinding::bind(&a).unwrap();
                let mut db = Client::connect(&url, NoTls).unwrap();
                barrier.wait();
                execute_atomic(&mut db, &approval, &a, "3").unwrap()
            })
        })
        .collect();
    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|x| **x == AtomicOutcome::Applied)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|x| **x == AtomicOutcome::AlreadyApplied)
            .count(),
        1
    );
    let mut db = Client::connect(&url, NoTls).unwrap();
    let version: i64 = db
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version, 4);
    db.execute(
        "DELETE FROM cer_postgres_effects WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute(
        "DELETE FROM cer_operation_journal WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn unsupported_operation_cannot_mutate_or_consume_identity() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("unsupported-{}", std::process::id());
    let mut db = Client::connect(&url, NoTls).unwrap();
    setup(&mut db);
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
        &[&id],
    )
    .unwrap();
    let mut a = action(&id, &id);
    a.operation = "delete".into();
    let approval = ApprovalBinding::bind(&a).unwrap();
    assert_eq!(
        execute_atomic(&mut db, &approval, &a, "3").unwrap(),
        AtomicOutcome::UnsupportedOperation
    );
    assert_eq!(
        inspect_durable_outcome(&mut db, &a).unwrap(),
        DurableOutcome::Absent
    );
    let version: i64 = db
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version, 3);
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn independent_operations_on_same_version_cannot_both_succeed() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("race-{}", std::process::id());
    let mut db = Client::connect(&url, NoTls).unwrap();
    setup(&mut db);
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
        &[&id],
    )
    .unwrap();
    drop(db);
    let barrier = Arc::new(Barrier::new(2));
    let handles: Vec<_> = (0..2)
        .map(|n| {
            let url = url.clone();
            let resource = id.clone();
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                let a = action(&format!("{resource}-op-{n}"), &resource);
                let approval = ApprovalBinding::bind(&a).unwrap();
                let mut db = Client::connect(&url, NoTls).unwrap();
                barrier.wait();
                execute_atomic(&mut db, &approval, &a, "3").unwrap()
            })
        })
        .collect();
    let outcomes: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == AtomicOutcome::Applied)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|o| **o == AtomicOutcome::StateChanged)
            .count(),
        1
    );
    let mut db = Client::connect(&url, NoTls).unwrap();
    let version: i64 = db
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version, 4);
    for n in 0..2 {
        db.execute(
            "DELETE FROM cer_postgres_effects WHERE operation_id = $1",
            &[&format!("{id}-op-{n}")],
        )
        .unwrap();
        db.execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&format!("{id}-op-{n}")],
        )
        .unwrap();
    }
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn rejected_admission_does_not_consume_operation_identity() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("admission-{}", std::process::id());
    let mut db = Client::connect(&url, NoTls).unwrap();
    setup(&mut db);
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
        &[&id],
    )
    .unwrap();
    let a = action(&id, &id);
    let approval = ApprovalBinding::bind(&a).unwrap();
    assert_eq!(
        execute_atomic(&mut db, &approval, &a, "4").unwrap(),
        AtomicOutcome::Rejected
    );
    assert_eq!(
        inspect_durable_outcome(&mut db, &a).unwrap(),
        DurableOutcome::Absent
    );
    assert_eq!(
        execute_atomic(&mut db, &approval, &a, "3").unwrap(),
        AtomicOutcome::Applied
    );
    let mut different = a.clone();
    different.arguments = ActionArguments::from_json_str(r#"{"value":2}"#).unwrap();
    assert_eq!(
        inspect_durable_outcome(&mut db, &different).unwrap(),
        DurableOutcome::IdentityConflict
    );
    db.execute(
        "DELETE FROM cer_postgres_effects WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute(
        "DELETE FROM cer_operation_journal WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn failed_version_compare_is_durable_and_not_retried() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("failed-{}", std::process::id());
    let mut db = Client::connect(&url, NoTls).unwrap();
    setup(&mut db);
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 5, '{}'::jsonb)",
        &[&id],
    )
    .unwrap();
    let a = action(&id, &id);
    let approval = ApprovalBinding::bind(&a).unwrap();
    assert_eq!(
        execute_atomic(&mut db, &approval, &a, "3").unwrap(),
        AtomicOutcome::StateChanged
    );
    assert_eq!(
        inspect_durable_outcome(&mut db, &a).unwrap(),
        DurableOutcome::Failed
    );
    assert_eq!(
        execute_atomic(&mut db, &approval, &a, "5").unwrap(),
        AtomicOutcome::ExistingFailed
    );
    let version: i64 = db
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version, 5);
    db.execute(
        "DELETE FROM cer_postgres_effects WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute(
        "DELETE FROM cer_operation_journal WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}

#[test]
fn backend_termination_before_commit_rolls_back_all_effects() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let id = format!("terminated-{}", std::process::id());
    let a = action(&id, &id);
    let approval = ApprovalBinding::bind(&a).unwrap();
    let mut worker = Client::connect(&url, NoTls).unwrap();
    setup(&mut worker);
    worker
        .execute(
            "INSERT INTO cer_resources (resource_id, version, payload)
             VALUES ($1, 3, '{}'::jsonb)",
            &[&id],
        )
        .unwrap();
    let pid: i32 = worker
        .query_one("SELECT pg_backend_pid()", &[])
        .unwrap()
        .get(0);
    let mut tx = worker.transaction().unwrap();
    tx.execute(
        "INSERT INTO cer_operation_journal (operation_id, fingerprint, status)
         VALUES ($1, 'interrupted', 'PENDING')",
        &[&id],
    )
    .unwrap();
    tx.execute(
        "UPDATE cer_resources SET version = 4 WHERE resource_id = $1",
        &[&id],
    )
    .unwrap();
    let mut controller = Client::connect(&url, NoTls).unwrap();
    let terminated: bool = controller
        .query_one("SELECT pg_terminate_backend($1)", &[&pid])
        .unwrap()
        .get(0);
    assert!(terminated);
    assert!(tx.commit().is_err());
    drop(worker);
    let mut recovered = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        inspect_durable_outcome(&mut recovered, &a).unwrap(),
        DurableOutcome::Absent
    );
    let version: i64 = recovered
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(version, 3);
    assert_eq!(
        execute_atomic(&mut recovered, &approval, &a, "3").unwrap(),
        AtomicOutcome::Applied
    );
    assert_eq!(
        inspect_durable_outcome(&mut recovered, &a).unwrap(),
        DurableOutcome::Succeeded
    );
    recovered
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    recovered
        .execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}
