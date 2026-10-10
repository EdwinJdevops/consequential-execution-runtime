use cer_core::{ActionArguments, ConditionalOutcome, ConditionalTarget, ProposedAction};
use cer_postgres_adapter::PostgresTarget;
use postgres::{Client, NoTls};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn competing_writers_cannot_both_commit_same_version() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL integration test: CER_TEST_DATABASE_URL unset");
        return;
    };
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin.batch_execute(
        "CREATE TEMP TABLE IF NOT EXISTS ignored_local_only (id INTEGER); \
         CREATE TABLE IF NOT EXISTS cer_resources (resource_id TEXT PRIMARY KEY, version BIGINT NOT NULL, payload JSONB NOT NULL)",
    )
    .unwrap();
    let id = format!("cer-concurrency-{}", std::process::id());
    admin
        .execute(
            "INSERT INTO cer_resources(resource_id, version, payload) VALUES ($1, 1, '{}'::jsonb)",
            &[&id],
        )
        .unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let url = url.clone();
        let id = id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let client = Client::connect(&url, NoTls).unwrap();
            let mut target = PostgresTarget::new(client);
            let action = ProposedAction {
                action_id: "same-version-test".into(),
                resource_id: id,
                operation: "update".into(),
                arguments: ActionArguments::from_json_str(r#"{"value":2}"#).unwrap(),
                observed_state: "1".into(),
                policy_version: "p1".into(),
            };
            barrier.wait();
            target.compare_and_execute(&action, "1").unwrap()
        }));
    }
    barrier.wait();
    let outcomes: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().unwrap())
        .collect();
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ConditionalOutcome::Applied)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == ConditionalOutcome::StateChanged)
            .count(),
        1
    );
    let row = admin
        .query_one(
            "SELECT version FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 2);
    admin
        .execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}
