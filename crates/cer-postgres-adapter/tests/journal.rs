use cer_postgres_adapter::journal::{admit, Admission, SCHEMA};
use postgres::{Client, NoTls};
use std::sync::{Arc, Barrier};
use std::thread;

#[test]
fn concurrent_duplicate_admission_and_conflicting_intent() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        eprintln!("skipping PostgreSQL journal test: CER_TEST_DATABASE_URL unset");
        return;
    };
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin.batch_execute(SCHEMA).unwrap();
    let id = format!("journal-{}", std::process::id());
    let barrier = Arc::new(Barrier::new(3));
    let mut handles = Vec::new();
    for _ in 0..2 {
        let url = url.clone();
        let id = id.clone();
        let barrier = Arc::clone(&barrier);
        handles.push(thread::spawn(move || {
            let mut client = Client::connect(&url, NoTls).unwrap();
            barrier.wait();
            admit(&mut client, &id, "fingerprint-A").unwrap()
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
            .filter(|outcome| **outcome == Admission::New)
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|outcome| **outcome == Admission::Existing)
            .count(),
        1
    );
    assert_eq!(
        admit(&mut admin, &id, "fingerprint-B").unwrap(),
        Admission::Conflict
    );
    let row = admin
        .query_one(
            "SELECT count(*) FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 1);
    admin
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
}
