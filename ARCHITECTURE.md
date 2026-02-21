# Architecture

## Section 1: The Double Entry Constraint

Every transaction must balance to zero. `Sum(Debits) - Sum(Credits) == 0`. If not, the system must return `TransactionError::AccountingImbalance`.

## Section 2: Concurrency

The system uses Optimistic Locking (Versioning) for account balances to avoid database deadlocks and ensure high throughput.

## Section 3: The Catalog

Products are treated as data, not code. For instance, a 'Gold Saver' is defined as a JSON configuration rather than a hard-coded class.

## Dataflow Diagram

```text

[ External Clients (Mobile, Web, ATMs) ]
                                     │
                                     ▼  (HTTP / JSON)
    ┌──────────────────────────────────────────────────────────────────┐
    │                              keva-api                            │
    │                   (Transport / REST / gRPC)                      │
    └────────────────────────────────┬─────────────────────────────────┘
                                     │
                                     ▼  (Domain Commands / Rust Structs)
    ┌──────────────────────────────────────────────────────────────────┐
    │                            keva-accounts                         │
    │               (The Orchestrator / State Management)              │
    └──────────────┬───────────────────────────────────┬───────────────┘
                   │                                   │
         1. Checks Product Rules            2. Sends Validated Entries
                   │                                   │
                   ▼                                   ▼
    ┌─────────────────────────────┐     ┌──────────────────────────────┐
    │        keva-catalog         │     │         keva-ledger          │
    │    (The Product Engine)     │     │    (The Source of Truth)     │
    │                             │     │                              │
    │ - Interest Configurations   │     │ - Double-Entry Math          │
    │ - Fee Structures            │     │ - Immutable Journal          │
    │ - Overdraft Limits          │     │ - Optimistic Concurrency     │
    └─────────────────────────────┘     └──────────────┬───────────────┘
                                                       │
                                                       ▼  (SQLx / OCC)
                                      ==================================
                                      [(      PostgreSQL Database     )]
                                      [  -> account_balances (State)   ]
                                      [  -> journal_entries (Log)      ]
                                      ==================================

```

## The Architecture Breakdown (How Data Flows)

To make this diagram actionable, let's trace a single action: A customer deposits 1,000 NPR.

***1. The Adapter Layer (keva-api)***

***Action***: Receives an HTTP POST request with a JSON payload ({ "account_id": "uuid-123", "amount": "1000.00" }).

***Responsibility***: It validates the JSON, authenticates the token, maps the request to a Keva domain command, and passes it down. It knows nothing about banking math.

***2.  The Orchestrator (keva-accounts)***

***Action***: Receives the internal command to deposit 1,000 NPR into uuid-123.

***Responsibility***: This is the traffic cop.

- It looks up the account to find its assigned Product ID.
- It queries keva-catalog: "Does Product X allow deposits?"
- If yes, it constructs a JournalEntry (Debit Cash, Credit Customer Account).
- It sends the JournalEntry to keva-ledger.

***3. The Configuration Engine (keva-catalog)***

***Action***: Responds to keva-accounts with the rules for the specific account's product.

***Responsibility***: Pure configuration. It holds definitions like "Gold Savings Account: Max Deposit = 100,000, Interest = 5%." It does not touch the database containing user balances.

***4. The Source of Truth (keva-ledger)***

***Action***: Receives the JournalEntry.

***Responsibility***: The strict, immutable core.

- It asserts Sum(Debits) == Sum(Credits).
- If the math is perfect, it attempts to write to PostgreSQL using Optimistic Concurrency Control.
- If successful, it returns an Ok(()) back up the chain.
