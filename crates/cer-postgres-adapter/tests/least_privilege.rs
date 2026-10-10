use cer_core::{ActionArguments, ApprovalBinding, ProposedAction};
use cer_postgres_adapter::transactional::{
    execute_atomic, inspect_durable_outcome, AtomicOutcome, DurableOutcome,
};
use postgres::{Client, NoTls};

#[test]
fn restricted_runtime_cannot_delete_or_rewrite_effect_evidence() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let mut owner = Client::connect(&url, NoTls).unwrap();
    owner.batch_execute("SELECT pg_advisory_lock(72451639)")
        .unwrap();
    owner
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS cer_resources (
                resource_id TEXT PRIMARY KEY, version BIGINT NOT NULL, payload JSONB NOT NULL
            );
            ALTER TABLE cer_resources ADD COLUMN IF NOT EXISTS last_operation_id TEXT;",
        )
        .unwrap();
    owner
        .batch_execute(cer_postgres_adapter::journal::SCHEMA)
        .unwrap();
    owner
        .batch_execute(include_str!(
            "../migrations/0004_atomic_effect_evidence.sql"
        ))
        .unwrap();
    owner
        .batch_execute(include_str!("../migrations/0005_revoke_public_access.sql"))
        .unwrap();
    owner.batch_execute("SELECT pg_advisory_unlock(72451639)")
        .unwrap();

    // Test-only role; the CI database's owner is permitted to administer roles.
    // A unique role avoids collision with concurrent test jobs.
    let role = format!("cer_runtime_{}", std::process::id());
    let id = format!("privilege-{}", std::process::id());
    let role_sql = format!(
        "CREATE ROLE {role} LOGIN PASSWORD 'test_only';
         GRANT CONNECT ON DATABASE cer_test TO {role};
         GRANT USAGE ON SCHEMA public TO {role};
         GRANT SELECT, INSERT, UPDATE ON cer_operation_journal TO {role};
         GRANT SELECT, INSERT ON cer_postgres_effects TO {role};
         GRANT SELECT, UPDATE ON cer_resources TO {role};"
    );
    owner.batch_execute(&role_sql).unwrap();
    owner
        .execute(
            "INSERT INTO cer_resources (resource_id, version, payload)
             VALUES ($1, 3, '{}'::jsonb)",
            &[&id],
        )
        .unwrap();

    let mut restricted_config: postgres::Config = url.parse().unwrap();
    restricted_config.user(&role).password("test_only");
    let mut runtime = restricted_config.connect(NoTls).unwrap();
    let rights = runtime
        .query_one(
            "SELECT
                has_table_privilege(current_user, 'cer_operation_journal', 'DELETE'),
                has_table_privilege(current_user, 'cer_postgres_effects', 'UPDATE'),
                has_table_privilege(current_user, 'cer_postgres_effects', 'DELETE')",
            &[],
        )
        .unwrap();
    assert!(!rights.get::<_, bool>(0));
    assert!(!rights.get::<_, bool>(1));
    assert!(!rights.get::<_, bool>(2));

    let action = ProposedAction {
        action_id: id.clone(),
        resource_id: id.clone(),
        operation: "update".into(),
        arguments: ActionArguments::from_json_str(r#"{"value":1}"#).unwrap(),
        observed_state: "3".into(),
        policy_version: "p1".into(),
    };
    let approval = ApprovalBinding::bind(&action).unwrap();
    assert_eq!(
        execute_atomic(&mut runtime, &approval, &action, "3").unwrap(),
        AtomicOutcome::Applied
    );
    assert_eq!(
        inspect_durable_outcome(&mut runtime, &action).unwrap(),
        DurableOutcome::Succeeded
    );
    assert!(runtime
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .is_err());
    assert!(runtime
        .execute(
            "UPDATE cer_postgres_effects SET fingerprint = 'forged'
             WHERE operation_id = $1",
            &[&id],
        )
        .is_err());
    assert!(runtime
        .execute(
            "DELETE FROM cer_postgres_effects WHERE operation_id = $1",
            &[&id],
        )
        .is_err());
    drop(runtime);

    assert_eq!(
        inspect_durable_outcome(&mut owner, &action).unwrap(),
        DurableOutcome::Succeeded
    );
    owner
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    owner
        .execute(
            "DELETE FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap();
    owner
        .batch_execute(&format!("DROP ROLE {role}"))
        .unwrap();
}
