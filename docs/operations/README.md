# Operations & Incident Response

This folder serves as the central hub for the operational aspects of Keva,
including DevOps, disaster recovery, and system failure mitigation.

As a high-performance banking kernel, Keva handles immense throughput and highly
concurrent requests. This documentation covers the edge case playbook,
highlighting how the architecture handles network timeouts, OCC (Optimistic
Concurrency Control) crashes, and idempotency failures.

## 📖 Edge Case Playbook

The Edge Case Playbook details real-world scenarios, business impact,
operational flows, and technical resolutions for complex distributed system
failures:

- **[0001: The Microsecond Double-Spend](./edge-cases/0001-the-microsecond-double-spend.md)**
  - Resolving race conditions and double-spending attempts using OCC.
- **[0002: Network Timeout Replays & Idempotency Key Misuse](./edge-cases/0002-network-timeout-replays-idempotency-key-misuse.md)**
  - Handling client-side retries and preventing duplicate ledger entries.
- **[0003: The Double-Click Problem](./edge-cases/0003-double-click-problem.md)**
  - Dropping parallel overlapping requests from impatient users using claim
    checks.
