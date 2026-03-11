# System Design: Native Rust CDC Audit Worker

## 1. System Objective

The `keva-worker` is a highly-available, background Rust daemon. Its primary responsibility is to act as an out-of-band security auditor. It streams the PostgreSQL Write-Ahead Log (WAL) to detect if a malicious actor (e.g., a superuser DBA) has physically altered the ledger history or balances on disk, bypassing the `keva-ledger` application layer.

## 2. Component Architecture

### 2.1 The Leader Election Engine

To support multi-container deployments without duplicating WAL consumption, the worker utilizes PostgreSQL Advisory Locks.

* **Active Leader:** Acquires a session-level lock (e.g., `pg_try_advisory_lock`), establishes the replication connection, and consumes the WAL.
* **Passive Follower:** Fails to acquire the lock, enters a sleep loop, and polls periodically to take over if the Active Leader crashes.

### 2.2 The Publication Filter

To prevent network I/O bottlenecks, the worker relies on native PostgreSQL Publications. 

* The worker only subscribes to `keva_audit_state` (`INSERT`, `UPDATE` on `ledger_state`) and `keva_audit_history` (`UPDATE`, `DELETE` on `journal_entry`).

### 2.3 The `pgoutput` Decoder

The worker uses `tokio-postgres` and `postgres-types` to read the raw binary stream. It manually decodes the proprietary `pgoutput` format into native Rust structs, extracting only the cryptographic fields (`entry_hash`, `current_balance`, `previous_state_hash`, etc.).

### 2.4 The Dual-Validation Pipeline

The decoded structs are passed into the `keva-ledger` domain for strict verification:

1.  **Cause Validation:** Verify the historical `JournalEntry` hash against its `Postings`.
2.  **Effect Validation:** Verify the `LedgerState` hash using the validated `entry_hash`.
3.  **Action:** If validation succeeds, advance the Log Sequence Number (LSN) and persist it to the `worker_state` table. If validation fails, halt execution and fire a Sev-1 alert.

### 2.5 The Anti-Blindness Heartbeat

To prevent a DBA from quietly dropping the Publication, the worker continuously inserts a UUID into an `audit_heartbeat` table. If the worker does not see its own heartbeat echo back through the WAL stream within 10 seconds, it assumes the audit trail is severed and triggers an alarm.