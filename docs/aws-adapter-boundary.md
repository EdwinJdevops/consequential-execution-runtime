# AWS adapter: implementation boundary (not deployed)

The PostgreSQL transaction path does not apply to AWS. Do not reuse its atomicity claims for remote API calls.

## Initial operation

Choose one resource and one reversible, bounded mutation after an explicit account/region/permissions review. A candidate is tagging a dedicated test S3 bucket created for CER, not modifying existing production resources. The exact AWS API semantics and tag update race behavior must be verified before implementing the adapter.

## Required gates before writes

1. Confirm AWS account and region using STS and read-only API calls.
2. Use least-privilege credentials scoped to a CER-owned test resource; do not require administrator permissions.
3. Define a stable operation identity and persist the approved request before an external call.
4. Verify provider-supported idempotency behavior. Do not assume every AWS API accepts client tokens.
5. On timeout, read authoritative AWS state and compare the intended mutation. If provenance is unavailable, retain UNKNOWN rather than claiming success.
6. Exercise retries, network disconnects, concurrent attempts, stale approval, and interrupted writes.
7. Enforce bounded cost, cleanup, and a deny-by-default production-resource boundary.

## Current status

No AWS adapter has been implemented or deployed. No AWS write should occur until these gates are satisfied and a scoped integration test is reviewed.
