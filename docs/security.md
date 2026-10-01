# Security model

## Assumptions
The authenticated autonomous runtime is untrusted. Assume prompt injection, malicious or compromised tools/MCP servers, credential exposure, stale approvals, races, replay/theft, partial failure, authority expansion through delegation, compromised dependencies, human error, ambiguous serialized input, and attempts to bypass CER.

## Enforcement boundary
M2 is an execution-semantics kernel, not yet a complete security boundary. If the caller can directly reach the target with usable credentials, it can bypass CER. Production enforcement therefore requires at least one target-enforced mechanism: CER brokers/holds downstream credentials; the target requires a CER-issued assertion/capability; or native admission rejects mutations without CER authorization.

## Action identity
CER no longer trusts caller-supplied text as a canonical representation of action arguments. A structured proposed action is deterministically JCS-serialized and SHA-256 hashed before approval binding. The fingerprint binds the action instance, target, operation, arguments, observed state, and policy version.

The current API begins from an already-parsed `serde_json::Value`. Therefore M2 does not yet prove that arbitrary raw JSON from an untrusted network peer had no duplicate keys or lossy parse behavior. The future ingress parser must establish those properties before a fingerprint becomes authoritative at a network trust boundary.

## Capability requirements for later milestones
Capabilities should be short-lived, audience- and operation-bound, non-expanding under delegation, replay-resistant, revocable where the protocol permits, and preferably proof-of-possession bound. Capability signatures should bind the canonical action fingerprint rather than reimplement a second serialization scheme.

## Explicit non-claims
M2 does not prevent direct target access, detect malicious intent universally, cure prompt injection, guarantee exactly-once external effects, provide durable evidence, sign action fingerprints, or remain safe against a compromised administrator/executor that can bypass the enforcement boundary.
