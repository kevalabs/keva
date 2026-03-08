# Specification: PRD-LEDG-003 - Audit Traceability & Temporal State

**Traceability Links:** - [Base] PRD-LEDG-001
- [Enhancement] PRD-LEDG-003

## 1. Module Responsibilities

This specification updates the core domain entities to support temporal tracking and external audit correlation using strict Rust types, ensuring the ledger can be traced back to human intent without understanding human identity.



## 2. Domain Entities (Deltas)

### 2.1 `LedgerState` (Appended Fields)

* `created_at: chrono::DateTime<chrono::Utc>`
* `updated_at: chrono::DateTime<chrono::Utc>`

### 2.2 `JournalEntry` (Appended Fields)

* `created_at: chrono::DateTime<chrono::Utc>`
* `correlation_id: uuid::Uuid`
* `metadata: Option<serde_json::Value>` (Optional JSON payload for upstream compliance data).

### 2.3 `Posting` (Appended Fields)

* `created_at: chrono::DateTime<chrono::Utc>`

## 3. Execution Rules

### 3.1 Temporal Initialization

1. When a new `LedgerState` is created, `created_at` and `updated_at` MUST be set to the exact same current UTC time (`chrono::Utc::now()`).
2. When a `LedgerState` is mutated via `apply_journal_entry`, the `updated_at` field MUST be updated to `chrono::Utc::now()`.

### 3.2 Cryptographic Hash Update (Integration with PRD-LEDG-002)

Because the state representation has changed, the cryptographic hashing formula MUST be updated to include the temporal mutation.
* **Updated Hash Formula:** `SHA-256(previous_state_hash + current_balance + version + updated_at_unix_timestamp)`