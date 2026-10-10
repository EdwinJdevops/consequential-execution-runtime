# PostgreSQL commit-acknowledgment fault gate

## Current verified scope

CER's PostgreSQL adapter commits the journal, resource mutation and effect
attribution in one transaction. The integration suite exercises successful
commit/reconnect, rollback on connection termination before COMMIT, concurrent
workers, and replay with missing or inconsistent evidence.

**These tests do not establish lost-COMMIT-acknowledgment behavior.**
A successful `execute_atomic` followed by deliberately discarding its return
value is not equivalent to losing the PostgreSQL wire response.

## Required fault experiment (not yet passed)

1. Run PostgreSQL 16 behind a TCP proxy with a deterministic fault rule.
   Route only the test worker connection through the proxy. Use a separate
   direct connection for recovery and for independent observations.
2. Seed a uniquely named resource at version 3 and construct one approved
   operation identity. Begin `execute_atomic` through the proxy.
3. Let the proxy forward the entire COMMIT request to PostgreSQL. After the
   server completes COMMIT, suppress the corresponding server-to-client
   response and close the worker's connection. The proxy must prove which
   frontend message and backend response it intercepted; a blind timeout is
   insufficient.
4. Require the worker to observe a transport error, not `Applied`. Reconnect
   directly to PostgreSQL and call `inspect_durable_outcome`.
5. Require `Succeeded` with matching fingerprint, resource, and version
   transition evidence. Replay the same operation and require `AlreadyApplied`.
   Verify exactly one version increment and one effect row.
6. Repeat with termination before COMMIT and verify no durable journal/effect
   and unchanged resource; do not conflate the two failure modes.
7. Repeat with tampered/missing effect evidence and require
   `EvidenceMismatch`, never `AlreadyApplied`.

## Acceptance criteria

Capture proxy fault logs, PostgreSQL observations, CI job URL and commit SHA.
Do not claim the gate passed until the fault has been demonstrated against
the actual wire protocol and the integration test is reproducible in CI.

## Operational contract

A failed COMMIT call is an **unknown outcome**, not evidence of failure.
Do not immediately create a new operation identity. Reconnect and inspect
the original identity first. `Absent` is not proof that an arbitrary external
effect did not occur; this adapter's atomic PostgreSQL transaction is a
narrower guarantee than cross-service exactly-once execution.
