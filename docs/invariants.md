# Safety invariants

- **INV-001 Preconditions:** no protected effect executes before required preconditions are established.
- **INV-002 Exact binding:** approval binds action identity, target, canonical arguments, observed state, and policy context.
- **INV-003 Revalidation:** material target-state change invalidates stale approval.
- **INV-004 Ambiguity:** an unknown remote outcome is not classified as failure without evidence.
- **INV-005 Retry safety:** a potentially completed non-idempotent action is not blindly retried.
- **INV-006 Recovery truth:** CER never promises recovery the adapter cannot establish with evidence.
- **INV-007 Recovery enforcement:** recovery is itself a consequential execution.
- **INV-008 Evidence authority:** untrusted callers cannot mutate authoritative execution records.
- **INV-009 Receipts:** completed protected transitions eventually require immutable/durable execution receipts; persistence is not implemented in M0.
- **INV-010 Determinism:** an LLM is not authoritative for hard safety predicates.
- **INV-011 Non-expanding delegation:** delegated authority is monotonically attenuating.
- **INV-012 Fail-closed capability use:** expired, revoked, replayed, substituted, or stale capabilities cannot authorize execution.
