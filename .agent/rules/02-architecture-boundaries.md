---
trigger: always_on
---

# Boundary Enforcement: keva-api

When your current context or task involves modifying the `keva-api` crate:
1. You may only generate HTTP handlers (using Axum), Traefik routing logic, and payload validation (using `serde`).
2. You must enforce the Idempotency Key 24-hour expiration check before initiating any state-mutating requests.
3. You may NOT write raw SQL `INSERT` or `UPDATE` statements for the ledger inside this crate. You must call the orchestrator commands in `keva-accounts`.