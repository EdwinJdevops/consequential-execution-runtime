use cer_postgres_adapter::observation::{observe, Observation};
use postgres::{Client, NoTls};

#[test]
fn observes_resource_without_asserting_causation() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "CI must provide CER_TEST_DATABASE_URL"
        );
        return;
    };
    let mut db = Client::connect(&url, NoTls).unwrap();
    db.batch_execute(
        "CREATE TABLE IF NOT EXISTS cer_resources (
            resource_id TEXT PRIMARY KEY,
            version BIGINT NOT NULL,
            payload JSONB NOT NULL
        )",
    )
    .unwrap();
    let id = format!("observe-{}", std::process::id());
    db.execute(
        "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 7, '{\"ready\":false}'::jsonb)",
        &[&id],
    )
    .unwrap();
    assert_eq!(
        observe(&mut db, &id, 7, r#"{"ready":true}"#).unwrap(),
        Observation::NotObserved
    );
    db.execute(
        "UPDATE cer_resources SET version = 8, payload = '{\"ready\":true}'::jsonb
         WHERE resource_id = $1",
        &[&id],
    )
    .unwrap();
    assert_eq!(
        observe(&mut db, &id, 7, r#"{"ready":true}"#).unwrap(),
        Observation::ConsistentWithEffect
    );
    assert_eq!(
        observe(&mut db, &id, 5, r#"{"ready":true}"#).unwrap(),
        Observation::Inconclusive
    );
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}
