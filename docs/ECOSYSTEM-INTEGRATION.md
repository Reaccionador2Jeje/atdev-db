# AtDev DB — Ecosystem Integration Contract

Status: planning document. It does not change the current experimental SQL engine or claim production readiness.

## Role
AtDev DB is an independent experimental SQL service. CloudForge and AtDev Cloud may integrate through a narrow HTTP contract, but must not assume PostgreSQL compatibility.

## Safety boundaries
- Preserve loopback-first binding behavior.
- Require bearer-token protection for non-loopback deployments.
- Keep request and SQL size limits.
- Treat SQL input as untrusted and preserve parser and authorization checks.
- Document backups and restore before production use.

## Proposed service contract
- GET /health: liveness/readiness without secrets.
- POST /v1/query: authenticated query execution using the existing request shape.
- Stable JSON errors with machine-readable codes.
- Explicit API versioning before incompatible changes.
- Per-request limits and bounded execution time.

## Integration sequence
1. Add contract tests for health, authentication, invalid requests and supported SQL.
2. Test non-loopback startup refusing an absent token.
3. Document backup/restore and data-directory permissions.
4. Add a CloudForge adapter only after the API contract is tested.
5. Keep unsupported SQL features clearly documented.

## Not promised
- PostgreSQL wire-protocol compatibility.
- Distributed transactions or replication.
- Production-grade durability guarantees until implemented and tested.