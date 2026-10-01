# Safety invariants

- **INV-001 Preconditions:** no protected effect executes before required preconditions are established.
- **INV-002 Exact binding:** approval binds action identity, target, canonical arguments, observed state, and policy context.
- **INV-003 Revalidation:** material target-state change invalidates stale approval.
- **INV-004 Ambiguity:** an unknown remote outcome is not classified as failure without evidence.
- **INV-005 Retry safety:** a potentially completed non-idempotent action is not blindly retried.
- **INV-006 Recovery truth:** CER never promises recovery the adapter cannot establish with evidence.
- **INV-007 Recovery enforcement:** recovery is itself a consequential execution.
- **INV-008 Evidence authority:** untrusted callers cannot mutate authoritative execution records.
- **INV-009 Receipts:** completed protected transitions eventually require immutable/durable execution receipts; persistence is not implemented yet.
- **INV-010 Determinism:** an LLM is not authoritative for hard safety predicates.
- **INV-011 Non-expanding delegation:** delegated authority is monotonically attenuating.
- **INV-012 Fail-closed capability use:** expired, revoked, replayed, substituted, or stale capabilities cannot authorize execution.
- **INV-013 Effect declaration:** before execution, a protected effect declares mutation, retry, outcome-resolution, and recovery semantics.
- **INV-014 In-doubt truth:** failed reconciliation that cannot prove either effect or no-effect enters `IN_DOUBT`; it is not converted to success, ordinary failure, or an automatic retry.
- **INV-015 Idempotency scope:** reusing a provider idempotency key permits retry only for the identical canonical request and only while the adapter can establish that the provider's target-specific idempotency guarantee still applies.
