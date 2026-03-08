# Specification: PRD-LEDG-002 - Cryptographic Tamper Evidence

**Traceability Links:** - [Base] PRD-LEDG-001

- [Enhancement] PRD-LEDG-002
- [Architecture] ADR-002: Account-Level Cryptographic Tamper Evidence

## 1. Module Responsibilities

This specification defines the cryptographic invariants required to make `keva-ledger` tamper-evident. It treats the underlying storage as a hostile environment, requiring the Rust domain to mathematically prove the integrity of a `LedgerState` before mutating it.

## 2. Domain Entities & Value Objects (Deltas)

* **Constants:**
  * `GENESIS_HASH`: A static string of 64 zeros (`"0000000000000000000000000000000000000000000000000000000000000000"`).

* **`LedgerError` (Appended Variants):**
  * `CryptographicMismatch`: Thrown when the calculated state hash does not match the provided state hash.

* **`LedgerState` (Appended Fields):**
  * `previous_state_hash: String`: A 64-character SHA-256 hex string.
  * `current_state_hash: String`: A 64-character SHA-256 hex string.

## 3. Cryptographic Rules

### 3.1 Account Initialization (Genesis)

1. When a new `LedgerState` is instantiated, its `previous_state_hash` MUST be explicitly set to `GENESIS_HASH`.
2. Its initial `current_state_hash` MUST be calculated using the hashing formula defined in 3.4.

### 3.2 Preconditions (Tier 1 Inline Verification)

1. Before `apply_journal_entry` executes any mathematical changes to a `LedgerState`, it MUST verify the integrity of the row.
2. The engine MUST calculate the expected hash using the formula in 3.4, passing in the current state's `previous_state_hash`, `current_balance`, and `version`.
3. If the calculated hash does not exactly match the `current_state_hash` on the entity, the function MUST immediately abort and return `LedgerError::CryptographicMismatch`.

### 3.3 Postconditions (Hash Chaining)

1. Upon successful mutation of the balances and version, the state MUST be cryptographically locked.
2. The old `current_state_hash` MUST be moved into the `previous_state_hash` field.
3. A new `current_state_hash` MUST be calculated using the formula in 3.4 based on the newly updated fields.

### 3.4 The Hashing Formula

1. The hashing algorithm MUST be `SHA-256` (via the `sha2` crate).
2. The input payload MUST be a concatenated UTF-8 string in this exact order, with no spaces or delimiters: `{previous_state_hash}{current_balance}{version}`.
3. The output MUST be formatted as a lowercase 64-character hexadecimal string.