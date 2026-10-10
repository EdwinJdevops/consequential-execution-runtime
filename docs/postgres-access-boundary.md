# PostgreSQL access boundary

## Threat model

The atomic PostgreSQL adapter is correct only when the runtime is not
permitted to forge, delete, or rewrite historical evidence outside its
transactional execution path. The current adapter has **no database-enforced
append-only evidence policy**. Its evidence can be changed by the table owner,
superusers, or roles granted UPDATE/DELETE. The journal's foreign key
`ON DELETE CASCADE` can erase corresponding effect evidence.

Migration `0005_revoke_public_access.sql` removes grants to `PUBLIC` but
does **not** enforce this stronger boundary by itself. Existing grants to
named roles must be audited separately. Do not use a schema owner or superuser
as an application connection identity in production.

## Minimum deployment controls

- Apply migrations using a separate migration principal. Keep that principal
  out of the runtime environment.
- Use a dedicated, non-owner runtime role with no CREATEDB, CREATEROLE,
  SUPERUSER, BYPASSRLS or schema CREATE privilege. Never grant blanket
  ALL PRIVILEGES on CER tables.
- Give a recovery/observer role SELECT only on journal, effects and resources.
  This role must not inherit runtime mutation privileges.
- The current runtime writes journal status and effects directly. Therefore
  it still needs INSERT/UPDATE on journal and INSERT on effects. Preventing
  runtime forgery requires a future trusted, narrowly scoped database
  function or separate attestation service; ordinary table grants alone
  cannot prove an operation was authorized.
- Deny DELETE on journal and effects to runtime and observer roles. Ensure
  journal deletion and retention are handled by a separate, audited process;
  existing FK cascade means journal deletion destroys effect evidence.
- Audit role memberships, table ACLs, SECURITY DEFINER functions, search_path,
  and connection credentials before production deployment. Enable TLS and
  credential rotation appropriate to the deployment.
- Treat `journal::reconcile` as a caller-attested legacy boundary: it accepts
  a supplied outcome and reference without independently verifying the target.
  It must not be exposed to an untrusted caller or described as authenticated
  reconciliation.

## Verification queries

Run as the migration owner to inspect grants:

```sql
SELECT table_name, grantee, privilege_type
FROM information_schema.role_table_grants
WHERE table_schema = 'public'
  AND table_name IN ('cer_operation_journal', 'cer_postgres_effects', 'cer_resources')
ORDER BY table_name, grantee, privilege_type;
```

Run as the **actual application principal** and verify the expected results:

```sql
SELECT
  has_table_privilege(current_user, 'cer_operation_journal', 'DELETE') AS can_delete_journal,
  has_table_privilege(current_user, 'cer_postgres_effects', 'UPDATE') AS can_edit_effects,
  has_table_privilege(current_user, 'cer_postgres_effects', 'DELETE') AS can_delete_effects;
```

All three must be false for a restricted runtime identity. These checks do
not establish append-only or tamper-proof storage. The existing CI uses a
privileged test role for schema setup and cleanup, not a production role.
