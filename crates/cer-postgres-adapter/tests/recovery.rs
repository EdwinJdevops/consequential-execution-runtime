use cer_postgres_adapter::journal::{admit, claim, Admission, Claim, SCHEMA};
use postgres::{Client, NoTls};

#[test]
fn disconnect_after_claim_does_not_allow_reexecution() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(
            std::env::var("CI").is_err(),
            "CI must provide CER_TEST_DATABASE_URL"
        );
        return;
    };
    let id = format!("crash-{}", std::process::id());
    {
        let mut first = Client::connect(&url, NoTls).unwrap();
        first.batch_execute(SCHEMA).unwrap();
        assert_eq!(
            admit(&mut first, &id, "fingerprint").unwrap(),
            Admission::New
        );
        assert_eq!(
            claim(&mut first, &id, "fingerprint").unwrap(),
            Claim::Claimed
        );
        // Simulate a process crash after the durable claim, before recording an outcome.
    }
    let mut recovered = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        admit(&mut recovered, &id, "fingerprint").unwrap(),
        Admission::Existing
    );
    assert_eq!(
        claim(&mut recovered, &id, "fingerprint").unwrap(),
        Claim::NotClaimable
    );
    let row = recovered
        .query_one(
            "SELECT status FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    assert_eq!(row.get::<_, String>(0), "EXECUTING");
    recovered
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
}
