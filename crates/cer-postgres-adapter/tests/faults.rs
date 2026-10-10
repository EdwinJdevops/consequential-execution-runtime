use cer_core::{ActionArguments, ApprovalBinding, ProposedAction};
use cer_postgres_adapter::transactional::{
    execute_atomic, inspect_durable_outcome, AtomicOutcome, DurableOutcome,
};
use postgres::{Client, NoTls};
use std::sync::{Arc, Barrier};

fn setup(db: &mut Client) {
    db.batch_execute(
        "CREATE TABLE IF NOT EXISTS cer_resources (
            resource_id TEXT PRIMARY KEY, version BIGINT NOT NULL, payload JSONB NOT NULL
        );
        ALTER TABLE cer_resources ADD COLUMN IF NOT EXISTS last_operation_id TEXT;",
    )
    .unwrap();
    db.batch_execute(cer_postgres_adapter::journal::SCHEMA)
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
        "DELETE FROM cer_operation_journal WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}
