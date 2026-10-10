use cer_postgres_adapter::journal::{admit, claim, mark_unknown, Admission, Claim, SCHEMA};
use postgres::{Client, NoTls};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn only_one_concurrent_claim_and_unknown_blocks_replay() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL claim test: CER_TEST_DATABASE_URL unset");
        return;
    };
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin.batch_execute(SCHEMA).unwrap();
    let id = format!("claim-{}", std::process::id());
    assert_eq!(
        admit(&mut admin, &id, "fingerprint").unwrap(),
        Admission::New
    );
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let url = url.clone();
        let id = id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut client = Client::connect(&url, NoTls).unwrap();
            barrier.wait();
            claim(&mut client, &id, "fingerprint").unwrap()
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
            .filter(|outcome| **outcome == Claim::Claimed)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == Claim::NotClaimable)
            .count(),
        1
    );
    assert!(mark_unknown(&mut admin, &id).unwrap());
    assert!(!mark_unknown(&mut admin, &id).unwrap());
    assert_eq!(
        claim(&mut admin, &id, "fingerprint").unwrap(),
        Claim::NotClaimable
    );
    admin
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
}
