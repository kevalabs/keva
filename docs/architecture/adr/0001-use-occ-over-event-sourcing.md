# ADR 0001: Adopt Optimistic Concurrency Control (OCC) over Event Sourcing for Core Ledger

**Status:** Accepted  
**Date:** 2026-02-27  

## 1. Context and Problem Statement

The Keva core banking system is designed to support high-throughput financial transactions (targeting 50,000+ TPS) to accommodate both retail banking volume and high-density batch processing (e.g., corporate payroll, property management mass-refunds).

Traditionally, relational databases (like PostgreSQL) face a hard mathematical ceiling when dealing with "Hot Keys"—a single account receiving thousands of simultaneous updates. Because ACID compliance requires row-level serialization (locking the row for the duration of the disk write), a hot account is mathematically capped at roughly 1,000 TPS, resulting in severe API latency and connection exhaustion.

To bypass this database lock, we initially designed an **Event Sourced, Lock-Free In-Memory State Machine** paired with a **CQRS** read model. While this theoretical architecture easily cleared 100,000 TPS, it introduced massive cognitive load and operational complexity, including the need for group-commit disk batching, eventual consistency management (Read-Your-Own-Writes), and Bitemporal state synchronization for the product catalog.

## 2. Considered Options

* **Option 1: Event Sourcing + CQRS.** Completely bypass the database lock using an in-memory Ring Buffer, pushing immutable events to an append-only log (e.g., Kafka/Redpanda), and projecting state to a Read Replica.
* **Option 2: PostgreSQL + Optimistic Concurrency Control (OCC) + Virtual Clearing Accounts.** Rely on standard relational database transactions but fundamentally alter the domain's business rules to eliminate the hot-key bottleneck natively.

## 3. Decision Outcome

We are proceeding with **Option 2: PostgreSQL + OCC + Virtual Clearing Accounts**. 

Instead of solving the hot-key problem with complex infrastructure (Event Sourcing), we will solve it using banking domain rules. Keva will enforce a strict routing policy:

1. Standard, isolated transactions (`N <= 100`) are processed synchronously using standard PostgreSQL `sqlx` transactions and Optimistic Concurrency Control.
2. High-density batch transactions (`N > 100`) hitting a single source account are intercepted. The total lump sum is atomically moved into a temporary **Virtual Clearing Account**.
3. A pool of background Tokio workers processes the individual transfers from the Virtual Clearing Account to the destination accounts in parallel using `SELECT ... FOR UPDATE SKIP LOCKED`.

## 4. Rationale

* **Maximum Code Leverage:** By encapsulating standard double-entry math inside a pure Rust crate (`kevalabs-ledger-core`) backed by PostgreSQL, the exact same ledger engine can be deployed across Tier-1 core banking instances, lightweight property management SaaS, and other enterprise projects without requiring a heavy Kafka/CQRS infrastructure footprint.
* **Operational Simplicity:** We eliminate the need to manage distributed consensus, projection workers, and complex API timeout logic for eventual consistency. The infrastructure is reduced to standard PostgreSQL, PgBouncer, and Patroni.
* **Zero Row Contention:** Because the Tokio workers debit from an isolated Virtual Clearing Account and credit distinct, individual retail accounts, PostgreSQL can process the batch in parallel at maximum disk speed without row-lock contention.

## 5. Consequences

* **Positive:** Drastically reduced cognitive load for the engineering team. Lower infrastructure costs. Standard DBA tooling and SQL can be used for reporting and audits. Immediate, strong read consistency.
* **Positive:** Provides a natural mechanism for exception handling. Failed batch transactions (e.g., blocked destination accounts) can be cleanly routed to a static Branch Suspense GL rather than executing complex, automated clawbacks.
* **Negative/Mitigation:** The application layer must strictly enforce the batch-routing rule. If a client manages to loop 10,000 individual synchronous HTTP requests against a single account (bypassing the batch API), the system will still experience row-lock degradation. We must mitigate this via API rate-limiting and enforcing batch endpoints for corporate clients.