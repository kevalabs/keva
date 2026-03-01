# Balance Mechanics & Ledger Math

This document defines the strict mathematical realities of account balances
within Keva and how they interact with transaction validation.

## 1. Current Balance vs. Available Balance

Every account in the Keva ledger simultaneously tracks two distinct balance
realities.

### The Current Balance (Ledger Balance)

The Current Balance represents the absolute, settled mathematical truth of the
account at any given microsecond.

- **Calculation:** It is the strict sum of all historical `postings` tied to the
  account.
- **Usage:** Used for End-of-Day interest accrual, official bank statements, and
  regulatory reporting. It does not fluctuate based on pending authorizations.

### The Available Balance (Spendable Balance)

The Available Balance represents the user's immediate purchasing power.

- **Calculation:** `Current Balance - (Pending Holds / Liens) + (Overdraft Limit)`
- **Usage:** Used strictly for transaction validation. When a user attempts to
  withdraw cash or swipe a finPOS terminal, the core banking engine evaluates
  the request exclusively against the Available Balance.

## 2. The Ledger Isolation Strategy (Holds)

Pending authorizations are never stored as a numeric column on the `accounts`
table. They are isolated in a separate `account_holds` table.

- **Why:** This strictly isolates temporary metadata mutations from core ledger
  mutations. It guarantees that the Optimistic Concurrency Control (OCC)
  `version` on the `accounts` table only increments when actual settled money
  moves, preventing false-positive transaction failures at the POS terminal.
