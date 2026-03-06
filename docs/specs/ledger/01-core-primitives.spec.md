# Specification: PRD-LEDG-001 - Core Math and Double-Entry Primitives

**Traceability Link:** https://docs.google.com/document/d/e/2PACX-1vRYdpSRcSqGRH4cfnsr_iVHEUL3ab6JXg-vtbsQFm5BgS3gdbwXpGOgJJwKQQZGqSyOLfEYC4cnB3Pa/pub

**Target Module:** `keva-ledger`

**File to Generate:** `crates/keva-ledger/src/domain.rs`

## 1. Execution Rules for the AI Agent

When implementing this specification, the AI MUST adhere to the following constraints:

1. **Strict Types:** Use `i64` for all monetary values. Floating-point types (`f32`, `f64`) are strictly forbidden. Use `uuid::Uuid` for identifiers.
2. **No I/O:** Do not include `sqlx`, database connections, or HTTP logic. This module is purely an in-memory mathematical state transition engine.
3. **Test-Driven:** You MUST generate exhaustive property-based tests using the `proptest` crate before writing the implementation. The tests must actively attempt to violate the Invariants, Preconditions, and Postconditions defined below.

## 2. Domain Entities & Value Objects (The Shape)

The AI must implement the following structures:

* **`Direction`:** Enum (`Debit`, `Credit`).
* **`LedgerError`:** Enum deriving `thiserror::Error` with variants: 
  * `ImbalancedJournalEntry`
  * `InsufficientFunds`
  * `AccountNotFound`
  * `ZeroAmountPosting`
  * `ArithmeticOverflow`
* **`LedgerState`:**
  * `id: Uuid`
  * `current_balance: i64`
  * `pending_holds: i64`
  * `overdraft_limit: i64`
  * `version: i32`
  * *Method:* `pub fn available_balance(&self) -> Result<i64, LedgerError>` (Formula: `current_balance - pending_holds + overdraft_limit`).
* **`Posting`:**
  * `account_id: Uuid`
  * `amount: i64`
  * `direction: Direction`
  * `remark: Option<String>`
* **`JournalEntry`:**
  * `id: Uuid`
  * `description: String`
  * `timestamp: chrono::DateTime<chrono::Utc>`
  * `postings: Vec<Posting>`

## 3. The Domain Service: `apply_journal_entry`

The AI must implement a pure function with the exact signature:
`pub fn apply_journal_entry(entry: &JournalEntry, mut states: std::collections::HashMap<Uuid, LedgerState>) -> Result<std::collections::HashMap<Uuid, LedgerState>, LedgerError>`

### 3.1 Invariants (Absolute Laws)

1. **Positive Amounts:** Every `Posting.amount` MUST be strictly `> 0`. If a `Posting` contains `0` or a negative number, return `LedgerError::ZeroAmountPosting`.
2. **Double-Entry Balance:** The sum of all `amount`s where `direction == Debit` MUST exactly equal the sum of all `amount`s where `direction == Credit` within the `JournalEntry`.

### 3.2 Preconditions (Checked before any mutation occurs)

1. The `JournalEntry` must satisfy the Double-Entry Balance invariant. If it does not, return `LedgerError::ImbalancedJournalEntry`.
2. Every `account_id` referenced in the `postings` array MUST exist in the provided `states` map. If any are missing, return `LedgerError::AccountNotFound`.
3. The function must evaluate the *entire* `JournalEntry` for available balance violations before applying *any* mutations. (Evaluate the net impact of all postings on a single account).
4. **Overflow Protection:** If calculating the `available_balance` or applying the mutation results in an `i64` integer overflow, the function MUST abort and return `LedgerError::ArithmeticOverflow`.

### 3.3 Postconditions (The resulting state)

1. **Mathematical Mutation:** * A `Debit` strictly decreases the `current_balance` of the target `LedgerState`.
   * A `Credit` strictly increases the `current_balance` of the target `LedgerState`.
2. **Limit Enforcement:** The resulting `current_balance` of any mutated `LedgerState` MUST NOT drop below `-(overdraft_limit - pending_holds)`. If it does, the entire operation is aborted and returns `LedgerError::InsufficientFunds`.
3. **Optimistic Concurrency:** Every `LedgerState` that was mutated MUST have its `version` integer incremented by exactly `1`. Unmodified states returned in the map must retain their original version.