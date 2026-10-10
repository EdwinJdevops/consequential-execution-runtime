# PostgreSQL journal: current recovery contract

## Proven guarantees (subject to CI)

- PostgreSQL enforces one operation ID per journal row.
- Conflicting fingerprints for an existing ID are rejected.
- A conditional PENDING -> EXECUTING update allows only one successful claim.
- Reconnecting after a durable claim cannot claim that ID again.
- UNKNOWN and terminal rows are not eligible for another claim.
- Reconciliation stores a caller-supplied evidence reference and permits only EXECUTING/UNKNOWN -> SUCCEEDED/FAILED.

## Explicit limitations

The journal and resource mutation use separate connections. They are **not** one atomic transaction. An execution can commit its resource mutation while the journal remains EXECUTING. Treat this as an ambiguous outcome, not a failed mutation.

The current `reconcile` function does **not** authenticate or validate evidence references. It is a low-level storage API, not a production reconciliation authority. Do not expose it to untrusted callers or automatically mark operations SUCCEEDED/FAILED based solely on an evidence string.

A journal status of SUCCEEDED written by the current executor is not a durable proof of external AWS state. The current adapter targets PostgreSQL only. No AWS adapter, provider-specific idempotency, operation correlation, or verified recovery is implemented.

Do not automatically reset EXECUTING/UNKNOWN to PENDING. No blind retries. A reconciliation worker must read the authoritative target, correlate the observed effect to the operation, and record its observation before deciding a terminal outcome. Where correlation is impossible, require manual intervention.

## Schema upgrades

For new databases, provision `journal::SCHEMA` once using a controlled migration. For databases provisioned before `evidence_reference`, apply `crates/cer-postgres-adapter/migrations/0002_reconciliation_evidence.sql` before using reconciliation.

## Production gate

The implementation is **not production-ready**. Required before AWS mutations: independent evidence verification; crash-window fault injection including lost commit acknowledgement; authorization of reconciliation; operational audit; and a scoped AWS integration test with explicit cost and permission boundaries.

## Read-only PostgreSQL observation

`observation::observe` compares the authoritative resource version and JSON payload against the proposed effect. `ConsistentWithEffect` means only that the state matches; it **does not establish that CER caused the change**. `NotObserved` is not proof the effect never happened (it could have been reverted). `Inconclusive` requires investigation. None of these observations authorizes an automatic terminal reconciliation. Reliable attribution requires a durable operation marker committed atomically with the resource mutation or a provider-issued operation identifier.
