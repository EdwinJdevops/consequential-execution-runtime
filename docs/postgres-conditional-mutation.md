# PostgreSQL conditional mutation: implementation contract

CER must not use a separate SELECT followed by an unconditional UPDATE. For a
PostgreSQL-owned resource, the adapter must issue a single conditional mutation:

```sql
UPDATE cer_resources
SET payload = $3, version = version + 1
WHERE resource_id = $1 AND version = $2
RETURNING version;
```

One returned row means the expected version matched and the transaction
performed the update. Zero rows means the expected version was stale or the
resource was absent; the adapter must not report success.

The transaction must commit before the adapter reports Applied. A lost
connection or timeout during COMMIT is an ambiguous outcome, not proof of
failure. In that case the adapter must return Unknown and reconcile using a
durable operation identity before any retry. Never blindly reissue a
non-idempotent mutation.

## Concurrent-writer test (required before adapter acceptance)

1. Create a resource at version 1 in a disposable PostgreSQL database.
2. Start two independent connections, each attempting the conditional UPDATE
   with expected version 1.
3. Assert exactly one committed update returns one row, and the other returns
   zero rows; the final version is 2.
4. Repeat with the connection interrupted around COMMIT; record that the
   outcome cannot be inferred from the client error alone.
5. Verify that direct writers without the same conditional protocol are outside
   CER's guarantee. Restrict database credentials accordingly.

## Important scope boundary

The current in-memory coordinator tests validate its protocol against a fake
target. They do not prove this PostgreSQL behavior. No live PostgreSQL
integration test or production adapter has been executed yet.
