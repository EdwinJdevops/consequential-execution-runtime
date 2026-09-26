# Security model

## Assumptions
The authenticated autonomous runtime is untrusted. Assume prompt injection, malicious or compromised tools/MCP servers, credential exposure, stale approvals, races, replay/theft, partial failure, authority expansion through delegation, compromised dependencies, human error, and attempts to bypass CER.

## Enforcement boundary
M0 is an execution-semantics kernel, not yet a complete security boundary. If the caller can directly reach the target with usable credentials, it can bypass CER. Production enforcement therefore requires at least one target-enforced mechanism: CER brokers/holds downstream credentials; the target requires a CER-issued assertion/capability; or native admission rejects mutations without CER authorization.

## Capability requirements for later milestones
Capabilities should be short-lived, audience- and operation-bound, non-expanding under delegation, replay-resistant, revocable where the protocol permits, and preferably proof-of-possession bound. Canonicalization must be defined before signatures/capability hashes become security claims.

## Explicit non-claims
M0 does not prevent direct target access, detect malicious intent universally, cure prompt injection, guarantee exactly-once external effects, or remain safe against a compromised administrator/executor that can bypass the enforcement boundary.
