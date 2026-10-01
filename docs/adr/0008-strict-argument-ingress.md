# ADR 0008: Strict JSON argument ingress
Status: Accepted for M2

## Context
Canonical hashing after ordinary JSON parsing is insufficient at an untrusted boundary because JSON objects may contain duplicate member names. Different parsers or downstream components can retain the first occurrence, the last occurrence, or reject the input. A fingerprint must be computed over the same structured value that execution consumes.

## Decision
CER introduces `ActionArguments` as the only argument type accepted by `ProposedAction`.

Untrusted JSON must enter through `ActionArguments::from_json_str` or `ActionArguments::from_json_slice`. The parser:
- recursively rejects duplicate object member names before map construction;
- treats escaped and unescaped spellings of the same member name as duplicates;
- rejects trailing JSON values;
- applies CER's JCS/I-JSON profile before constructing `ActionArguments`.

Programmatically constructed typed data may enter through `ActionArguments::from_serializable`, which applies the same profile validation.

The inner `serde_json::Value` is private and exposed read-only. This prevents ordinary callers from mutating argument data after validation.

## Execution rule
Adapters must execute from the validated structured `ActionArguments` representation (or a deterministic typed conversion from it), not from the original raw JSON bytes. That ensures approval identity and execution semantics refer to the same parsed value.

## Non-claims
This does not make JSON itself a domain schema. Target adapters still need operation-specific validation, type/range checks, authorization, state preconditions, and target-specific effect contracts.

JCS intentionally does not Unicode-normalize strings. CER preserves that behavior.

## Consequences
Duplicate-key ambiguity is removed from CER's supported raw JSON ingress path. SDKs and future network handlers must not pre-parse untrusted JSON with a permissive parser and then present the resulting value as though CER had validated the original bytes.
