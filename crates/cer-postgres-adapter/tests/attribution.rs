use cer_postgres_adapter::attribution::{
    apply_attributed, observe_attribution, AttributedObservation,
};
use postgres::{Client, NoTls};

#[test]
fn attribution_survives_reconnect_and_rejects_stale_writes() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "CI must provide CER_TEST_DATABASE_URL"
        );
        return;
    };
    let id = format!("attribution-{}", std::process::id());
    {
        let mut db = Client::connect(&url, NoTls).unwrap();
        db.batch_execute(
            "CREATE TABLE IF NOT EXISTS cer_resources (
                resource_id TEXT PRIMARY KEY,
                version BIGINT NOT NULL,
                payload JSONB NOT NULL
            );
            ALTER TABLE cer_resources ADD COLUMN IF NOT EXISTS last_operation_id TEXT;",
        )
        .unwrap();
        db.execute(
            "INSERT INTO cer_resources (resource_id, version, payload)
             VALUES ($1, 4, '{}'::jsonb)",
            &[&id],
        )
        .unwrap();
        assert!(apply_attributed(&mut db, &id, 4, r#"{"ok":true}"#, "op-1").unwrap());
        assert!(!apply_attributed(&mut db, &id, 4, r#"{"ok":false}"#, "op-2").unwrap());
    }
    let mut db = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        observe_attribution(&mut db, &id, "op-1").unwrap(),
        AttributedObservation::Proven
    );
    assert_eq!(
        observe_attribution(&mut db, &id, "op-2").unwrap(),
        AttributedObservation::DifferentOperation
    );
    let row = db
        .query_one(
            "SELECT version, payload::text FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 5);
    let payload: serde_json::Value =
        serde_json::from_str(&row.get::<_, String>(1)).unwrap();
    assert_eq!(payload, serde_json::json!({"ok":true}));
    db.execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}
