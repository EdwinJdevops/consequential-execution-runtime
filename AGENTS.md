# CER Engineering Contract

CER is a consequential-execution runtime. It is not an agent framework, IAM replacement, LLM evaluator, generic workflow engine, Terraform/Kubernetes policy product, or UI demo.

## Trust model
Treat the autonomous caller, prompts, tool output, plugins/MCP servers, dependencies, and networks as untrusted. An LLM may propose an action but cannot establish authorization, target state, approval validity, retry safety, recovery capability, successful execution, or recovery completion.

CER is not an enforcement boundary while a protected caller retains an alternate credential/network path to mutate the target directly. A production boundary must own/broker downstream authority, require a CER-issued assertion enforced by the target, or use native admission that rejects mutations lacking valid CER authorization.

## Mandatory invariants
1. No protected effect executes until required preconditions are satisfied.
2. Approval binds exact action, target, canonical arguments, observed state, and policy context.
3. Material state change invalidates prior approval.
4. Unknown remote outcome is never silently classified as failure.
5. Non-idempotent EXECUTION_UNKNOWN actions are never blindly retried.
6. Recovery claims require target-specific evidence.
7. Recovery actions are consequential actions and are safety-enforced.
8. Untrusted callers cannot mutate authoritative execution evidence.
9. Completed protected transitions produce durable evidence/receipts once persistence exists.
10. LLM output is outside the trusted computing base for hard safety decisions.
11. Delegated authority must not expand beyond its parent authority.
12. Replay, substitution, revocation, expiry, and stale-state checks fail closed.
13. Every protected effect declares retry, outcome-resolution, and recovery semantics before execution.
14. If reconciliation cannot prove effect or no-effect, execution enters IN_DOUBT rather than success or ordinary failure.
15. Reusing a provider idempotency key authorizes retry only for the identical canonical request and only while the adapter can establish that the provider's target-specific idempotency guarantee still applies.

## Engineering rules
- Read relevant ADRs and threat model before semantic changes.
- No speculative refactors or future abstractions.
- No unsupported production/security claims.
- Never claim exactly-once external side effects without a proven protocol.
- Preserve EXECUTION_UNKNOWN and IN_DOUBT as first-class states.
- Never expose a generic rollback guarantee.
- Adapter retry/recovery claims require target-specific evidence and conformance tests.
- Idempotency-key reuse alone is not evidence of retry safety; target scope and provider semantics matter.
- Prefer deterministic tests; use fault injection for partial failures.
- Any new security-sensitive dependency needs explicit justification.
- No unsafe Rust in core/adapters without an approved ADR.
- Scope changes to state machine, trust boundary, recovery, delegation, persistence, or adapter contract require an ADR.
