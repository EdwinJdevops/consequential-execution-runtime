# Consequential Execution Runtime (CER)

CER is an execution-assurance runtime research project for consequential autonomous operations across heterogeneous external systems.

The current engineering focus is deliberately narrow: explicit ambiguous-outcome handling, exact approval/state binding, effect contracts, retry/reconciliation semantics, recovery contracts, and target-semantic adapters. CER does not yet claim a bypass-resistant production enforcement boundary.

## Current status

M0 is verified on `main`: the Rust workspace passes formatting, Clippy with warnings denied, and workspace tests in GitHub Actions.

M1 introduces executable effect contracts. Before a protected mutation executes, its adapter must declare mutation, retry, outcome-resolution, and recovery semantics. Unknown outcomes remain first-class; an unreconcilable outcome cannot be converted into success or ordinary failure and cannot be blindly retried.

See `docs/problem.md`, `docs/invariants.md`, `docs/security.md`, `docs/threat-model.md`, and the ADRs before implementation work.
