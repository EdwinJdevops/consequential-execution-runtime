# Threat model

## Assets
Execution authority; canonical action identity; validated action arguments; target resources; approval evidence; observed-state evidence; effect contracts; recovery evidence; execution receipts; delegation lineage.

## Adversaries and failures
- compromised or prompt-injected autonomous caller;
- malicious tool/MCP server or dependency;
- caller holding direct target credentials;
- confused deputy and resource substitution;
- approval/capability replay or theft;
- stale approval and TOCTOU;
- non-canonical or ambiguous JSON representation;
- duplicate object names at an untrusted raw JSON boundary;
- downstream re-parsing of raw arguments after approval;
- precision-changing numeric input;
- Unicode noncharacter or normalization confusion;
- delegation that expands authority;
- revocation/expiry race and clock skew;
- duplicate/non-idempotent execution;
- remote effect followed by lost acknowledgement;
- process crash around external commit;
- network partition;
- idempotency-key substitution, loss, expired validity, or target-scope mismatch across retry;
- incorrect adapter declaration of retry, reconciliation, or recovery semantics;
- outcome that cannot be resolved after reconciliation attempts;
- recovery evidence corruption/staleness;
- recovery racing with newer external state;
- policy downgrade/change during execution;
- compromised control-plane administrator.

## Required adversarial tests over time
Target substitution, operation substitution, argument substitution, action-instance substitution, stale-state approval, policy-version mismatch, object-key reorderings, duplicate raw JSON keys, escaped duplicate keys, nested duplicate keys, Unicode normalization differences, unsafe numeric inputs, replay, duplicate request, lost acknowledgement, unknown-result retry, idempotency-key substitution, idempotency-scope mismatch, unreconcilable unknown outcome, recovery-semantics mismatch, recovery failure, state change before recovery, delegation expansion, expired/revoked capability, and direct-bypass attempts once credential mediation exists.
