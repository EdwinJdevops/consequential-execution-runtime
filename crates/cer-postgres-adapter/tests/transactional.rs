use cer_core::{ActionArguments, ApprovalBinding, ProposedAction};
use cer_postgres_adapter::transactional::{execute_atomic, AtomicOutcome};
use postgres::{Client, NoTls};

#[test]
fn journal_and_mutation_commit_together_and_replay_is_read_only() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "CI must provide CER_TEST_DATABASE_URL"
        );
        return;
    };
    let mut db = Client::connect(&url, NoTls).unwrap();
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
    let id = format!("atomic-{}", std::process::id());
    let resource = format!("resource-{id}");
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
        &[&resource],
    )
    .unwrap();
    let action = ProposedAction {
        action_id: id.clone(),
        resource_id: resource.clone(),
        operation: "update".into(),
        arguments: ActionArguments::from_json_str(r#"{"value":1}"#).unwrap(),
        observed_state: "3".into(),
        policy_version: "p1".into(),
    };
    let approval = ApprovalBinding::bind(&action).unwrap();
    assert_eq!(
        execute_atomic(&mut db, &approval, &action, "3").unwrap(),
        AtomicOutcome::Applied
    );
    drop(db);
    let mut db = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        execute_atomic(&mut db, &approval, &action, "4").unwrap(),
        AtomicOutcome::AlreadyApplied
    );
    let row = db
        .query_one(
            "SELECT version, last_operation_id FROM cer_resources WHERE resource_id = $1",
            &[&resource],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 4);
    assert_eq!(row.get::<_, String>(1), id);
    let mut conflicting = action.clone();
    conflicting.arguments = ActionArguments::from_json_str(r#"{"value":2}"#).unwrap();
    let other_approval = ApprovalBinding::bind(&conflicting).unwrap();
    assert_eq!(
        execute_atomic(&mut db, &other_approval, &conflicting, "4").unwrap(),
        AtomicOutcome::IdentityConflict
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
    db.execute(
        "DELETE FROM cer_resources WHERE resource_id = $1",
        &[&resource],
    )
    .unwrap();
}
