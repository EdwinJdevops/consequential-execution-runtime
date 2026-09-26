# ADR 0005: Resource-semantic adapter boundary
Status: Accepted provisionally

CER adapters expose target semantics rather than generic tool/HTTP calls. M0 includes resolve, prepare, execute, and reconcile. The interface is intentionally minimal and must be revised if real PostgreSQL semantics cannot fit without target-specific leakage into the core.
