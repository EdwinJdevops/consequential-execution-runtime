# ADR 0007: Canonical action identity
Status: Accepted for M2

## Context
Approval, replay prevention, evidence receipts, and future signatures need one deterministic byte representation of the exact action being authorized. Treating caller-supplied text as "canonical" is unsafe: semantically identical JSON objects can differ in property order or insignificant serialization details, while distinct Unicode strings must not be silently normalized into the same value.

RFC 8785 defines the JSON Canonicalization Scheme (JCS) for deterministic UTF-8 JSON suitable for cryptographic hashing. JCS requires I-JSON-compatible input, recursively sorted object properties, ECMAScript-compatible number serialization, and preservation of Unicode string data without normalization.

## Decision
CER fingerprints a structured `ProposedAction` as:

`SHA-256(JCS(ProposedAction))`

The exact approval fingerprint includes:
- action instance identifier;
- target resource identifier;
- operation;
- validated structured arguments;
- observed-state token;
- policy version.

Including `action_id` is intentional: an approval is bound to one action instance, not to every future action with identical business semantics. A separate semantic-deduplication identity may be introduced later if evidence demonstrates a need; it must not weaken approval replay resistance.

CER applies a stricter numeric profile than generic JSON. Integer arguments outside ±(2^53-1) are rejected and must be represented as strings or another explicitly typed representation if exact interchange is required.

Rust strings cannot contain lone surrogate code points, but they can contain Unicode noncharacters. CER rejects Unicode noncharacters in argument keys and string values.

Raw JSON arguments are parsed through the duplicate-key-aware `ActionArguments` ingress defined by ADR 0008 before they can be placed in a proposed action.

## Dependency decision
M2 pins `serde_json_canonicalizer` 0.3.2 for RFC 8785 serialization and `sha2` 0.10.9 for SHA-256. CI checks the workspace on both current stable Rust and the declared Rust 1.82 MSRV.

## Consequences
Approval matching is content-addressed rather than a collection of ad hoc string comparisons. Future capabilities and receipts can bind to the same fingerprint, but signing is not part of M2.
