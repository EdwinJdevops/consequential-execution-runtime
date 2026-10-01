# Architecture — M2

The CER kernel separates proposed actions, canonical action identity, exact approval binding, execution state, effect semantics, recovery capability, and target adapters.

## Canonical action identity
A structured `ProposedAction` is serialized using RFC 8785 JCS and hashed with SHA-256. The fingerprint covers the action instance ID, target resource, operation, arguments, observed-state token, and policy version. Approval binding stores this fingerprint rather than trusting a caller-provided "canonical arguments" string.

CER deliberately does not normalize Unicode before fingerprinting. It also rejects argument integers outside the exact I-JSON interoperable integer range and rejects Unicode noncharacters. The current kernel starts from `serde_json::Value`; a strict raw-wire parser that detects duplicate object names before ordinary JSON parsing is still required.

## Effect settlement
State progression is explicit. `EXECUTION_UNKNOWN` represents the epistemic condition where CER cannot prove whether a remote effect occurred. It can proceed only to reconciliation, never directly to completion or automatic retry. If reconciliation cannot establish either effect or no-effect, the execution enters `IN_DOUBT`.

An `EffectContract` is declared before execution. The contract records:
- whether an operation is read-only or mutating;
- whether an unknown outcome may be repeated, requires reuse of a stable provider idempotency key, or must be reconciled first;
- whether outcome resolution is available through provider lookup and/or postcondition verification;
- the declared recovery semantics.

The adapter boundary remains target-semantic rather than generic HTTP/tool execution. Preparation returns a `PreparedEffect` that couples target-specific prepared data with its effect contract and prepared recovery capability. CER rejects a prepared effect if its runtime recovery capability contradicts the contract's declared recovery semantics.

M2 still has no durable journal, network server, credential broker, policy engine, cloud integration, cryptographic capability/signature format, or signed evidence receipt. Those omissions are intentional and remain security-relevant.
