# ADR 0004: Stateless Read-After-Write Consistency via Cryptographic Headers

**Status:** Accepted  
**Date:** 2026-02-28

## 1. Context and Problem Statement

Following ADR 0002 (Database Read/Write Splitting), all ledger mutations are
routed to the Primary PostgreSQL node, while read queries are routed to
asynchronous Read Replicas. This introduces the problem of replication lag: if a
user executes a transfer and immediately refreshes their balance, the query may
hit a Replica that has not yet applied the transaction, showing the user a stale
balance and causing panic.

We need a mechanism to enforce "Read-Your-Own-Writes" (RYOW) consistency—routing
a user's read requests to the Primary node for a short window immediately after
they perform a write. However, introducing a centralized state store (like
Redis) or relying on load-balancer Sticky Sessions violates our core principle
of keeping the Rust API tier completely stateless and uniformly load-balanced.

## 2. Considered Options

- **Option 1: Distributed State (Redis).** The API writes a short-lived
  `recent_write=true` flag to Redis. Read requests check Redis to determine
  routing. (Rejected: Introduces new stateful infrastructure).
- **Option 2: Load Balancer Sticky Sessions.** Traefik pins a user's session to
  a specific API container, allowing that container to use local RAM for the
  flag. (Rejected: Causes uneven CPU distribution across the Swarm during
  high-volume batch processing).
- **Option 3: Stateless Cryptographic Time-Bound Tokens (HMAC).** The API
  delegates state storage to the client by sending a cryptographically signed
  timestamp header.

## 3. Decision Outcome

We are proceeding with **Option 3: Stateless Cryptographic Time-Bound Tokens
(HMAC)**, applied strictly to high-frequency, user-facing domains (e.g., account
balances, recent transactions).

**The Implementation Flow:**

1. **The Write:** Upon a successful ledger mutation, the API records the exact
   server timestamp, cryptographically signs it using HMAC-SHA256 with a secret
   server key, and returns it to the client via a custom HTTP header:
   `X-Keva-Sync-Token: <timestamp>.<signature>`.
2. **The Client Contract:** The client application (Mobile App, finPOS terminal)
   must store this token and attach it to subsequent `GET` requests for bounded
   domains (like `/balance`).
3. **The Read Middleware:** When a read request arrives, the API middleware
   intercepts the `X-Keva-Sync-Token`.
   - It recalculates the HMAC signature to verify the timestamp was not tampered
     with.
   - It evaluates the TTL: `Current Server Time - Token Timestamp`.
   - If the TTL is **< 3 seconds**, the query is routed to the **Primary Node**
     (guaranteeing consistency).
   - If the TTL is **>= 3 seconds**, or if the token is missing/invalid, the
     query is routed to the **Read Replica Pool**.

## 4. Rationale

- **Zero Server-Side State:** The API tier remains 100% stateless, allowing
  infinite horizontal scaling and perfect round-robin load balancing.
- **Self-Destructing State:** By relying on temporal decay rather than
  single-use nonces, we avoid the need for a database lookup to track "used"
  tokens. Malicious actors attempting to replay the token to artificially load
  the Primary node are mathematically cut off exactly 3 seconds after
  generation.
- **Infrastructure Simplicity:** We solve a complex distributed systems problem
  using pure mathematics and HTTP headers, keeping the infrastructure footprint
  strictly to PostgreSQL and Rust.

## 5. Consequences

- **Positive:** Perfect read-after-write consistency for the end user without
  sacrificing the performance benefits of read replicas.
- **Negative/Mitigation:** Requires strict compliance from front-end clients. If
  a third-party integrator or mobile developer fails to store and forward the
  `X-Keva-Sync-Token`, their users will experience replication lag. We must
  explicitly document this requirement in the Keva API specification.
