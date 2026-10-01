# Consequential Execution Runtime (CER)

CER is an execution-assurance runtime research project for consequential autonomous operations across heterogeneous external systems.

The current engineering focus is deliberately narrow: canonical action identity, exact approval/state binding, explicit ambiguous-outcome handling, effect contracts, retry/reconciliation semantics, recovery contracts, and target-semantic adapters. CER does not yet claim a bypass-resistant production enforcement boundary.

## Current status

M0 established and verified the initial Rust execution-safety kernel.

M1 added executable effect contracts, provider-aware retry decisions, and `IN_DOUBT` handling for outcomes reconciliation cannot prove.

M2 is establishing canonical action identity. Structured proposed actions are RFC 8785 JCS-serialized and SHA-256 hashed so approval binds one exact action instance rather than caller-asserted canonical text. CER applies a strict input profile for interoperable integers and Unicode noncharacters. A duplicate-key-aware raw ingress parser is not implemented yet, so M2 does not claim authoritative identity for arbitrary untrusted raw JSON bytes.

CI verifies formatting, Clippy with warnings denied, workspace tests on current stable Rust, and a separate compile check on the declared Rust 1.82 MSRV.

See `docs/problem.md`, `docs/invariants.md`, `docs/security.md`, `docs/threat-model.md`, and the ADRs before implementation work.
