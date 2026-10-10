//! Wire-level lost-COMMIT-ack regression against PostgreSQL's simple-query protocol.
//! Proxy suppresses the backend's COMMIT response after forwarding the request.
use cer_core::{ActionArguments, ApprovalBinding, ProposedAction};
use cer_postgres_adapter::transactional::{
    execute_atomic, inspect_durable_outcome, AtomicOutcome, DurableOutcome,
};
use postgres::{Client, NoTls};
use std::{
    io::{Read, Write},
    net::{Shutdown, TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

fn packet(stream: &mut TcpStream) -> std::io::Result<(u8, Vec<u8>)> {
    let mut head = [0_u8; 5];
    stream.read_exact(&mut head)?;
    let size = u32::from_be_bytes(head[1..5].try_into().unwrap()) as usize;
    if !(4..=16_777_216).contains(&size) {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid frame",
        ));
    }
    let mut frame = head.to_vec();
    frame.resize(size + 1, 0);
    stream.read_exact(&mut frame[5..])?;
    Ok((head[0], frame))
}

#[test]
fn commit_applied_but_ack_dropped_is_reconciled_without_second_effect() {
    let Ok(url) = std::env::var("CER_TEST_DATABASE_URL") else {
        assert!(std::env::var("CI").is_err(), "CI requires database");
        return;
    };
    let mut admin = Client::connect(&url, NoTls).unwrap();
    admin
        .batch_execute("SELECT pg_advisory_lock(72451639)")
        .unwrap();
    admin
        .batch_execute(
            "CREATE TABLE IF NOT EXISTS cer_resources (
           resource_id TEXT PRIMARY KEY, version BIGINT NOT NULL, payload JSONB NOT NULL
         );
         ALTER TABLE cer_resources ADD COLUMN IF NOT EXISTS last_operation_id TEXT;",
        )
        .unwrap();
    admin
        .batch_execute(cer_postgres_adapter::journal::SCHEMA)
        .unwrap();
    admin
        .batch_execute(include_str!(
            "../migrations/0004_atomic_effect_evidence.sql"
        ))
        .unwrap();
    admin
        .batch_execute("SELECT pg_advisory_unlock(72451639)")
        .unwrap();

    let id = format!("ack-loss-{}", std::process::id());
    admin
        .execute(
            "INSERT INTO cer_resources (resource_id, version, payload)
         VALUES ($1, 3, '{}'::jsonb)",
            &[&id],
        )
        .unwrap();
    let action = ProposedAction {
        action_id: id.clone(),
        resource_id: id.clone(),
        operation: "update".into(),
        arguments: ActionArguments::from_json_str(r#"{"value":1}"#).unwrap(),
        observed_state: "3".into(),
        policy_version: "p1".into(),
    };
    let approval = ApprovalBinding::bind(&action).unwrap();

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = listener.local_addr().unwrap().port();
    let suppressed = Arc::new(AtomicBool::new(false));
    let suppressed_proxy = Arc::clone(&suppressed);
    let proxy = thread::spawn(move || {
        let (mut downstream, _) = listener.accept().unwrap();
        downstream
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();
        let mut upstream = TcpStream::connect("127.0.0.1:5432").unwrap();
        upstream
            .set_read_timeout(Some(std::time::Duration::from_secs(15)))
            .unwrap();

        // Startup is length-prefixed, without a message-type byte.
        let mut startup_len = [0_u8; 4];
        downstream.read_exact(&mut startup_len).unwrap();
        let len = u32::from_be_bytes(startup_len) as usize;
        assert!((8..=65536).contains(&len));
        let mut startup = vec![0; len - 4];
        downstream.read_exact(&mut startup).unwrap();
        upstream.write_all(&startup_len).unwrap();
        upstream.write_all(&startup).unwrap();

        let commit_forwarded = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&commit_forwarded);
        let mut server_read = upstream.try_clone().unwrap();
        let mut client_write = downstream.try_clone().unwrap();
        let response = thread::spawn(move || {
            while let Ok((kind, frame)) = packet(&mut server_read) {
                if flag.load(Ordering::SeqCst) {
                    // Suppress all COMMIT reply frames including CommandComplete and ReadyForQuery.
                    if kind == b'Z' {
                        suppressed_proxy.store(true, Ordering::SeqCst);
                        let _ = client_write.shutdown(Shutdown::Both);
                        break;
                    }
                } else if client_write.write_all(&frame).is_err() {
                    break;
                }
            }
        });

        let mut commit_pending = false;
        while let Ok((kind, frame)) = packet(&mut downstream) {
            // postgres may issue COMMIT through extended Parse/Bind/Execute/Sync,
            // rather than a simple Query message.
            if kind == b'Q' || kind == b'P' {
                eprintln!("frontend sql kind={} bytes={:?}", kind, &frame[5..]);
            }
            let commit = (kind == b'Q' || kind == b'P')
                && frame[5..].windows(6).any(|bytes| bytes == b"COMMIT");
            if commit {
                commit_pending = true;
                commit_forwarded.store(true, Ordering::SeqCst);
            }
            if upstream.write_all(&frame).is_err()
                || (commit_pending && (kind == b'S' || kind == b'Q'))
            {
                break;
            }
        }
        response.join().unwrap();
    });

    let mut config: postgres::Config = url.parse().unwrap();
    config
        .ssl_mode(postgres::config::SslMode::Disable)
        .host("127.0.0.1")
        .port(port)
        .connect_timeout(std::time::Duration::from_secs(5));
    let mut worker = config.connect(NoTls).unwrap();
    let result = execute_atomic(&mut worker, &approval, &action, "3");
    drop(worker);
    proxy.join().unwrap();
    assert!(
        result.is_err(),
        "worker must not receive COMMIT acknowledgment: {result:?}"
    );
    assert!(
        suppressed.load(Ordering::SeqCst),
        "proxy did not suppress COMMIT ReadyForQuery"
    );

    let mut recovery = Client::connect(&url, NoTls).unwrap();
    assert_eq!(
        inspect_durable_outcome(&mut recovery, &action).unwrap(),
        DurableOutcome::Succeeded
    );
    assert_eq!(
        execute_atomic(&mut recovery, &approval, &action, "3").unwrap(),
        AtomicOutcome::AlreadyApplied
    );
    let row = recovery
        .query_one(
            "SELECT version, last_operation_id FROM cer_resources WHERE resource_id = $1",
            &[&id],
        )
        .unwrap();
    assert_eq!(row.get::<_, i64>(0), 4);
    assert_eq!(row.get::<_, String>(1), id);
    let count: i64 = recovery
        .query_one(
            "SELECT count(*) FROM cer_postgres_effects WHERE operation_id = $1",
            &[&id],
        )
        .unwrap()
        .get(0);
    assert_eq!(count, 1);
    recovery
        .execute(
            "DELETE FROM cer_operation_journal WHERE operation_id = $1",
            &[&id],
        )
        .unwrap();
    recovery
        .execute("DELETE FROM cer_resources WHERE resource_id = $1", &[&id])
        .unwrap();
}
