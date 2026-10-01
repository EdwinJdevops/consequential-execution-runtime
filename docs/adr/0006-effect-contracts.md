# ADR 0006: Effect contracts precede consequential execution
Status: Accepted provisionally for M1

## Context
A generic success/error return is insufficient for consequential external mutations. After request dispatch, a timeout may mean no remote effect, a committed effect with a lost acknowledgement, or an outcome the caller cannot determine. Retry safety also varies by target: some operations are repeatable, some are safe only when the same provider idempotency key is reused, and some require reconciliation before any retry.

Recovery semantics are similarly target-specific. A runtime must not infer that an operation is reversible merely because an inverse-looking API exists.

## Decision
Before a protected effect executes, its target adapter must provide an `EffectContract` declaring:
- mutation kind;
- retry semantics after an unknown result;
- available outcome-resolution semantics;
- recovery semantics.

Preparation returns a `PreparedEffect` coupling the contract to the target-specific prepared operation and prepared recovery capability. The adapter API rejects a prepared effect whose recovery capability contradicts the contract.

The kernel's retry decision for an unknown outcome is deterministic. Request substitution fails closed. Stable-idempotency-key retries require reuse of the same key. Operations requiring reconciliation cannot be retried directly. If the adapter declares that reconciliation is unavailable, direct retry is denied.

If reconciliation cannot prove effect or no-effect, the execution enters `IN_DOUBT`.

## Non-claims
An adapter declaration is not proof that the target really implements those semantics. Production adapters require target-specific conformance tests and evidence from official API behavior. M1 does not provide exactly-once execution, durable persistence, cryptographic contract identity, or bypass-resistant credential enforcement.

## Consequences
The adapter surface becomes stricter and slightly more expensive to implement. That cost is intentional: target-specific effect semantics are part of the trusted computing base and cannot safely be guessed by a model or generic HTTP wrapper.
