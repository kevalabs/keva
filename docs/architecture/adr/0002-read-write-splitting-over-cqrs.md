# ADR 0002: Adopt Database Read/Write Splitting over Strict CQRS

**Status:** Accepted  
**Date:** 2026-02-28  

## 1. Context and Problem Statement

In a core banking environment, the ratio of database reads to writes is massively
skewed. Passive actions (customers constantly refreshing their mobile apps to
check balances) can account for over 90% of the traffic. If all `SELECT` queries
are routed to the Primary PostgreSQL node, they will consume CPU and connection
limits, artificially capping the system's ledger mutation throughput (TPS).

Our initial design utilized strict Command Query Responsibility Segregation (CQRS)
with Event Sourcing, utilizing background projection workers to populate a
distinct Read database. Having dropped Event Sourcing (see ADR 0001), we needed
a new strategy to offload read traffic without re-introducing complex projection
infrastructure.

## 2. Considered Options

* **Option 1: Strict CQRS without Event Sourcing.** The API writes to the
  primary relational database, and an application-level background worker listens
  for database triggers or an outbox table to update a separate, read-optimized
  database (e.g., MongoDB, Elasticsearch).

* **Option 2: PostgreSQL Native Read/Write Splitting with Sticky Routing.** The
  API writes to the Primary PostgreSQL node and routes standard reads to
  PostgreSQL Read Replicas, utilizing native streaming replication.

## 3. Decision Outcome

We are proceeding with **Option 2: PostgreSQL Native Read/Write Splitting with
Sticky Routing**.

* All ledger mutations (`INSERT`, `UPDATE`) are routed strictly to the Primary
  Node.

* All standard balance inquiries (`SELECT`) are routed to the Read Replica pool.

* **Sticky Routing Mitigation:** To solve replication lag (where a user transfers
  money and instantly checks their balance before the replica catches up), the
  API layer will implement a "Primary-Read-After-Write" rule. When a user
  successfully mutates their ledger state, the API caches a short-lived flag
  (e.g., 3 seconds). Any read requests from that specific user within the
  3-second window are intercepted and routed directly to the Primary Node,
  ensuring immediate read-your-own-writes consistency.

## 4. Rationale

* **Zero Application-Level Synchronization:** Native PostgreSQL streaming
  replication handles data copying at the binary file level. We completely
  eliminate the need to write, monitor, and maintain custom Rust projection
  workers.

* **Schema Uniformity:** Both the Primary and Replica nodes share the exact same
  database schema, eliminating the cognitive load of maintaining separate data
  models for Commands and Queries.

* **Targeted Consistency:** 99% of "refresh spam" traffic is safely absorbed by
  the replicas. Only the users actively moving money query the Primary, perfectly
  balancing High Availability with strict ACID requirements.

## 5. Consequences

* **Positive:** Massive reduction in infrastructure complexity. We maintain
  standard SQL query capabilities across both read and write paths.

* **Negative/Mitigation:** The application layer (API Gateway) takes on the
  responsibility of intelligent routing. The API must reliably track the
  "recently written" state per user to prevent customer panic from eventual
  consistency on the replicas.
