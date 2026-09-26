# Problem

A single autonomous operation can perform individually authorized mutations across multiple independent systems while the overall execution is unsafe, stale, partially completed, unrecoverable, replayed, or ambiguous.

Per-system authorization answers whether a principal may invoke an operation. It does not prove that the compound state transition remains valid at execution time, that approval still applies to the observed state, that a partially completed operation can be reconciled, or that a claimed recovery mechanism exists.

CER investigates deterministic safety semantics around consequential external state transitions. The planner may be probabilistic; the execution decision is not delegated to the model.

## Non-goals
CER is not an IAM replacement, prompt-injection detector, malicious-intent oracle, LLM evaluator, generic workflow engine, Terraform policy engine, Kubernetes admission replacement, or observability backend.

## Unproven hypotheses
- A reusable core can normalize enough execution semantics across heterogeneous targets to be useful.
- Resource adapters will not contain nearly all useful logic.
- A bypass-resistant execution boundary can be integrated without unacceptable operational friction.
- Operators will prefer this boundary over existing workflow/change-control systems for some autonomous workloads.

## Kill conditions
Reconsider the thesis if useful semantics require knowing the whole workflow in advance; adapters absorb essentially all logic; callers retain unavoidable bypass authority; recovery cannot be modeled coherently; or existing platforms expose equivalent cross-system semantics natively.
