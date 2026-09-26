# Architecture — M0

The M0 kernel separates proposed actions, exact approval binding, execution state, recovery capability, and target adapters.

State progression is explicit. `EXECUTION_UNKNOWN` represents the epistemic condition where CER cannot prove whether a remote effect occurred. It can proceed only to reconciliation, never directly to completion or automatic retry.

The adapter boundary exposes target state resolution, preparation, execution outcome, and reconciliation. It deliberately distinguishes `ProvenNoEffect`, `ProvenEffect`, and `UnknownEffect`.

M0 has no persistence, network server, credential broker, policy engine, cloud integration, or cryptographic capability format. Those omissions are intentional and security-relevant.
