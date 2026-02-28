# ADR 0003: In-Memory Reference Caching via PostgreSQL NOTIFY

**Status:** Accepted  
**Date:** 2026-02-28

## 1. Context and Problem Statement

To achieve the target throughput of 5,000+ TPS, the ledger must evaluate product
rules (e.g., overdraft limits, transaction fees, tier restrictions) for every
single mutation. Executing a relational `SELECT` query against the `products`
and `rules` tables for every transaction introduces unacceptable database load
and network latency.

Because reference data (product configurations) changes very infrequently
compared to transactional data (account balances), it is an ideal candidate for
caching. However, introducing a distributed external cache (like Redis or
Memcached) violates our core architectural principle of minimizing stateful
infrastructure overhead. We need a mechanism to cache this data in-memory within
the Rust API nodes while guaranteeing strict, real-time cache invalidation
across the entire Docker Swarm when a Branch Manager updates a rule.

## 2. Considered Options

- **Option 1: Distributed Cache (Redis).** All API nodes read from a centralized
  Redis cluster.
- **Option 2: Time-to-Live (TTL) Local Cache.** API nodes cache rules in RAM for
  a short duration (e.g., 5 minutes). Changes take up to 5 minutes to propagate.
- **Option 3: Local In-Memory Cache with PostgreSQL `LISTEN/NOTIFY`.** API nodes
  cache rules entirely in RAM. PostgreSQL natively acts as a Pub/Sub message
  broker to instantly broadcast invalidation events to all connected API nodes
  when a rule changes.

## 3. Decision Outcome

We are proceeding with **Option 3: Local In-Memory Cache with PostgreSQL
`LISTEN/NOTIFY`**.

1. **State Loading:** On boot, each `keva-api` Tokio application loads the
   active Bitemporal product catalog into a high-performance, concurrent Rust
   in-memory data structure (e.g., `moka` or `arc_swap`).
2. **Transaction Evaluation:** The domain logic (`kevalabs-ledger-core`)
   evaluates transaction fees and limits using this local RAM cache. Network
   hops for reference data are completely eliminated, reducing read latency to
   nanoseconds.
3. **The Invalidation Trigger:** A PostgreSQL trigger is attached to the
   `products` and `product_rules` tables. Upon any `INSERT` or `UPDATE`, the
   database executes `NOTIFY product_updates, '{"product_id": "X"}';`.
4. **The Listener:** Each API node maintains a single, dedicated asynchronous
   database connection running `LISTEN product_updates`. Upon receiving the
   payload, the specific API node fetches the latest bitemporal row for Product
   X and atomically swaps it in its local RAM.

## 4. Rationale

- **Zero Infrastructure Sprawl:** We achieve distributed cache invalidation
  without deploying, monitoring, or maintaining a separate Redis or Kafka
  cluster. The entire state architecture remains strictly within standard
  PostgreSQL.
- **Maximum Performance:** Reading from a local Rust memory map is orders of
  magnitude faster than querying an external Redis instance over the network.
- **Bitemporal Synergy:** Because the product catalog is bitemporal (ADR
  000X/Previous Design), the cache does not need to handle complex, destructive
  updates. It simply appends the newly approved `effective_from` product state
  to the local RAM map, ensuring in-flight transactions are never evaluating
  partial rule states.

### 4.1 Strict Usage Boundaries

To prevent overloading the PostgreSQL notification queue, `LISTEN/NOTIFY` is
strictly governed by the following rules:

- **Use PostgreSQL NOTIFY for:** Low-frequency, globally critical state changes
  (Product Rules, System Halts, Feature Toggles).
- **Never use PostgreSQL NOTIFY for:** High-frequency, transient, user-specific
  data (Rate limiting, Session TTLs, Cache invalidation of individual
  transactions).

## 5. Consequences

- **Positive:** Unlocks 5,000+ TPS by entirely removing reference data lookups
  from the hot path. Keeps DevOps maintenance requirements at zero.
- **Negative/Mitigation:** Each API container requires one dedicated, persistent
  PostgreSQL connection strictly for the `LISTEN` command. This slightly reduces
  the available connections in the PgBouncer pool, which must be accounted for
  in capacity planning.
- **Negative/Mitigation:** If the network partition drops the `LISTEN`
  connection, the API node might miss an invalidation event. The Rust
  application must be engineered to automatically reconnect and execute a full
  background catalog sync upon restoring the connection.
