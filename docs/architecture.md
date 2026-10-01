# Architecture — M1

The CER kernel separates proposed actions, exact approval binding, execution state, effect semantics, recovery capability, and target adapters.

State progression is explicit. `EXECUTION_UNKNOWN` represents the epistemic condition where CER cannot prove whether a remote effect occurred. It can proceed only to reconciliation, never directly to completion or automatic retry. If reconciliation cannot establish either effect or no-effect, the execution enters `IN_DOUBT`.

M1 adds an `EffectContract` declared before execution. The contract records:
- whether an operation is read-only or mutating;
- whether an unknown outcome may be repeated, requires reuse of a stable provider idempotency key, or must be reconciled first;
- whether outcome resolution is available through provider lookup and/or postcondition verification;
- the declared recovery semantics.

The adapter boundary remains target-semantic rather than generic HTTP/tool execution. Preparation now returns a `PreparedEffect` that couples target-specific prepared data with its effect contract and prepared recovery capability. CER rejects a prepared effect if its runtime recovery capability contradicts the contract's declared recovery semantics.

M1 still has no persistence, network server, credential broker, policy engine, cloud integration, cryptographic capability format, or signed evidence receipt. Those omissions are intentional and remain security-relevant.
