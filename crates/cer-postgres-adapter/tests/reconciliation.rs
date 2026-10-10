use cer_postgres_adapter::journal::{
    admit, claim, mark_unknown, reconcile, Admission, Claim, Reconciliation, SCHEMA,
};
use postgres::{Client, NoTls};

#[test]
fn reconciliation_requires_evidence_and_preserves_terminal_outcome() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "CI must provide CER_TEST_DATABASE_URL"
        );
        return;
    };
    let mut db = Client::connect(&url, NoTls).unwrap();
    db.batch_execute(SCHEMA).unwrap();
    let id = format!("reconcile-{}", std::process::id());
    assert_eq!(admit(&mut db, &id, "fingerprint").unwrap(), Admission::New);
    assert_eq!(claim(&mut db, &id, "fingerprint").unwrap(), Claim::Claimed);
    assert!(mark_unknown(&mut db, &id).unwrap());
    assert_eq!(
        reconcile(&mut db, &id, "SUCCEEDED", "").unwrap(),
        Reconciliation::NotEligible
    );
    assert_eq!(
        reconcile(&mut db, &id, "UNVERIFIED", "reference").unwrap(),
        Reconciliation::NotEligible
    );
    assert_eq!(
        reconcile(&mut db, &id, "SUCCEEDED", "observation-123").unwrap(),
        Reconciliation::Recorded
    );
    assert_eq!(
        reconcile(&mut db, &id, "FAILED", "observation-456").unwrap(),
        Reconciliation::NotEligible
    );
    let row = db
        .query_one(
            "SELECT status, evidence_reference FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "SUCCEEDED");
    assert_eq!(row.get::<_, String>(1), "observation-123");
    assert_eq!(
        claim(&mut db, &id, "fingerprint").unwrap(),
        Claim::NotClaimable
    );
    db.execute(
        "DELETE FROM cer_operation_journal WHERE operation_id = $1",
        &[&id],
    )
    .unwrap();
}
