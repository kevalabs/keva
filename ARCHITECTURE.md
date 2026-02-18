# Architecture

## Section 1: The Double Entry Constraint

Write down: "Every transaction must balance to zero. Sum(Debits) - Sum(Credits) == 0. If not, return TransactionError::AccountingImbalance."

## Section 2: Concurrency

Write down: "We will use Optimistic Locking (Versioning) for account balances to avoid database deadlocks."

## Section 3: The Catalog

Write down: "Products are Data, not Code. A 'Gold Saver' is just a JSON configuration, not a hard-coded class."