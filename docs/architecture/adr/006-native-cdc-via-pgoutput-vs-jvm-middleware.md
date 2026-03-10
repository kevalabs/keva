# ADR-006: Native Rust CDC vs. JVM Middleware (Debezium/Kafka)

**Date:** 2026-03-10

**Status:** Accepted

**Context Area:** `keva-worker`, Infrastructure, Security Auditing

## 1. Context and Problem Statement

To detect intraday database tampering, Keva requires an event-driven Change Data Capture (CDC) pipeline. The industry default for CDC is deploying Debezium, Apache Kafka, and Zookeeper. We must decide whether to adopt this JVM-based middleware stack or build a custom native integration.

## 2. Decision

We explicitly reject the Debezium/Kafka stack. We will build a native Rust CDC listener inside the `keva-worker` crate using the `tokio-postgres` library to stream the `pgoutput` logical replication format. State (LSN cursors) will be managed in a local PostgreSQL utility table.

## 3. Consequences

### Positive

* **Zero Infrastructure Bloat:** Keva remains a single compiled binary alongside a PostgreSQL database, making it highly attractive for air-gapped or low-resource on-premise banking deployments.
* **Absolute Control:** By decoding `pgoutput` natively, we bypass JSON serialization overhead entirely, validating hashes at maximum byte-level efficiency.
* **Separation of Duties:** We achieve out-of-band security monitoring without relying on native database triggers (which a DBA can disable).

### Negative

* **Custom Decoder Maintenance:** The engineering team must maintain the raw binary parsing logic for `pgoutput`, a task normally abstracted by Debezium.
* **WAL Retention Risk:** If all worker nodes die, PostgreSQL will retain WAL files indefinitely waiting for LSN acknowledgment. Strict infrastructure monitoring on `pg_wal_size` is required to prevent disk exhaustion.
* **SaaS Multi-Tenancy Bottleneck:** This architecture is heavily optimized for single-tenant (Database-per-Tenant) deployments. If Keva pivots to a pooled multi-tenant SaaS model, a single Rust worker streaming a combined WAL for 1,000 banks will become a bottleneck, requiring a re-evaluation of Kafka.