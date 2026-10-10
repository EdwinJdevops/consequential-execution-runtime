# CER validation decisions — GPU and AWS investigation (2026-10-09)

## Disposition

The standalone GPU Evidence Gate product investigation is **closed**. Existing
SkyPilot, MLPerf, MLflow / Weights & Biases, and DVC capabilities overlap the
proposal. No validated unique market gap, buyer demand, savings, or material
competitor deficiency was established. Do not build a GPU scheduler, price
dashboard, or separate GPU product under CER.

The task-scoped agent-policy/envelope direction is **closed** as a standalone
CER differentiator. AWS AgentCore Policy and existing IAM/policy systems cover
significant authorization and policy enforcement needs. CER must not claim
novelty by duplicating them.

## Accepted CER boundary

CER owns **execution state, durable intent, and recovery assurance** for one
narrowly defined deployment workflow. It does not own generic agent
authorization or arbitrary agent actions. The first vertical slice must prove:

1. Durable, uniquely identified operations and idempotent state transitions.
2. Concurrency-safe admission and execution; replay cannot duplicate an effect.
3. Crash, timeout, and retry recovery, including unknown commit outcomes.
4. Independent reconciliation against actual AWS resource state, not only
   local logs or provider API acknowledgements.
5. Evidence-backed SUCCEEDED / FAILED / PARTIAL / UNKNOWN outcomes.
6. Audit integrity, fault injection, restart recovery, and safe partial-failure
   behavior before any production-readiness claim.

## Evidence discipline imported from GPU investigation

- Only compare reproducible, comparable workload/configuration observations.
- Separate measured runtime from estimates; record timing methodology.
- Distinguish quota exhaustion from actual capacity shortage.
- Cost conclusions require rates and billable durations.
- Reject unsupported provider rankings, savings claims, or certainty.
- These are **validation rules**, not a mandate to implement GPU features.

## Explicit exclusions for the initial slice

No GPU provisioning, GPU dashboards, Kubernetes platform, new authentication
system, AI agents, private-chat ingestion, arbitrary cloud actions, or new
authorization engine. Avoid infrastructure until an acceptance test requires it.

## Release gate

A passing Rust unit suite or PostgreSQL CAS test alone is insufficient.
Production claims require persisted operation identity, retry/replay controls,
crash/restart and partial-failure injection, AWS reconciliation, and audit
integrity tests with recorded outcomes.
