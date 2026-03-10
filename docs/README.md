# Keva Core Banking Platform

**System:** Keva Zero-Trust Ledger & Orchestration Engine
**Status:** Active Development
**Architecture:** Hexagonal, CQRS, Event-Driven (Native CDC)

Welcome to the Keva documentation root. This repository is strictly partitioned to separate business intent from infrastructure reality. Choose a domain below to navigate the system blueprints.

---

## 1. Domain (The "What" & The Business Rules)

Defines the ubiquitous language, business logic, and lifecycles independent of any technology stack. 
*(Location: `docs/domain/`)*

| Document | Core Focus |
| :--- | :--- |
| **[Domain Overview](./domain/README.md)** | Overview of core business logic, ledger mechanics, and lifecycles. |
| **[Account Lifecycle](./domain/account-lifecycle.md)** | State machine (Pending, Active, Frozen, Closed) and KYC gating. |
| **[Balance Mechanics](./domain/balance-mechanics.md)** | CQRS caching patterns and zero-sum validation rules. |
| **[End of Day Processing](./domain/end-of-day-processing.md)** | EOD sweeps, reconciliation, and ledger closing mechanics. |
| **[Glossary](./domain/glossary.md)** | The definitive ubiquitous language mapping for the engineering team. |

---

## 2. Architecture (The "How" & The "Why")

Defines the system design, infrastructure boundaries, and historical technical decisions.
*(Location: `docs/architecture/`)*

| Document | Core Focus |
| :--- | :--- |
| **[Architecture Overview](./architecture/README.md)** | High-level system topology and Hexagonal boundaries. |
| **[System Design: CDC Audit Worker](./architecture/cdc-audit-worker.md)** | Native PostgreSQL WAL streaming and Dual-Validation pipeline. |
| **[ADR Index](./architecture/adr/)** | The chronological log of all Architecture Decision Records (e.g., OCC over Event Sourcing, Native CDC vs. Kafka). |

---

## 3. Specifications (The Strict Rust Contracts)

Defines the absolute module-level constraints, cryptographic formulas, and data structures before code is written. 
*(Location: `docs/specs/`)*

| Document | Core Focus |
| :--- | :--- |
| **[01. Core Primitives](./specs/ledger/01-core-primitives.spec.md)** | The strict struct definitions for `JournalEntry`, `Posting`, and `LedgerState`. |
| **[02. Cryptography](./specs/ledger/02-cryptography.spec.md)** | The account-level SHA-256 hash chaining formula. |
| **[03. Audit Traceability](./specs/ledger/03-audit-traceability.spec.md)** | Temporal state constraints (UTC) and external `correlation_id` mapping. |
| **[04. Interlocking Cryptography](./specs/ledger/04-interlocking-cryptography.spec.md)** | Binding the `JournalEntry` hash directly into the `LedgerState` calculation. |

---

## 4. Operations (Runbooks & Edge Cases)

Defines deployment realities, disaster recovery, and known distributed systems anomalies.
*(Location: `docs/operations/`)*

| Document | Core Focus |
| :--- | :--- |
| **[Operations Overview](./operations/README.md)** | Deployment topologies and infrastructure monitoring targets. |
| **[Edge Case: Microsecond Double Spend](./operations/edge-cases/0001-the-microsecond-double-spend.md)** | Concurrency handling and OCC collision mitigation. |
| **[Edge Case: Network Timeouts & Idempotency](./operations/edge-cases/0002-network-timeout-replays-idempotency-key-misuse.md)** | Safe retry mechanics and API idempotency key abuse. |
| **[Edge Case: The Double Click Problem](./operations/edge-cases/0003-double-click-problem.md)** | Front-end debounce failures colliding with database locks. |