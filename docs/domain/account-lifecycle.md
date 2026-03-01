# Account Lifecycle & State Transitions

This document defines the strict business rules and ledger mutations required
for standard account lifecycle events within the Keva Core Banking system.

## 1. Account Closure (Real-Time Settlement)

**Business Context:** When a customer requests an account closure, the bank
legally owes the customer any interest accrued up to that exact day, even if the
scheduled batch capitalization (monthly/quarterly) has not yet occurred. The
system cannot wait for the batch job; it must perform a real-time settlement
before allowing the final withdrawal.

**The Ledger Rules:** An account can only transition to a `CLOSED` state if its
available balance is exactly `0.00` and its linked accrued liability is exactly
`0.00`.

### The Closure Workflow

```mermaid
sequenceDiagram
    participant UI as Keva Teller UI
    participant API as Keva API
    participant Replica as Postgres (Read)
    participant Primary as Postgres (Write)

    UI->>API: POST /accounts/123/close

    %% Step 1: Read Accrued Liability
    Note over API, Replica: 1. The Accrual Check
    API->>Replica: SELECT SUM(amount) FROM postings WHERE gl = 'ACCRUED_PAYABLE'
    Replica-->>API: 74 NPR (Unpublished Interest)

    %% Step 2: Settle Interest
    Note over API, Primary: 2. Real-Time Capitalization
    API->>Primary: INSERT journal_entry (Debit Liability 74, Credit Account 74)

    %% Step 3: Final Sweep
    Note over API, Primary: 3. The Final Sweep
    API->>Replica: SELECT balance FROM accounts WHERE id = '123'
    Replica-->>API: 50,074 NPR (Principal + Interest)
    API->>Primary: INSERT journal_entry (Debit Account 50,074, Credit Vault/Cash)

    %% Step 4: State Change
    Note over API, Primary: 4. The State Change
    API->>Primary: UPDATE accounts SET status = 'CLOSED' WHERE id = '123'
    Primary-->>API: Transaction Committed

    API-->>UI: 200 OK (Account Closed, Receipt Generated)

```

## Operational Invariants

- **Batch Job Safety:** The standard End-of-Day and End-of-Month batch jobs only
  query accounts where status = 'ACTIVE'. By closing the account in real-time,
  the async workers will naturally skip this account, preventing any
  double-payment of interest.

- **Atomic Execution:** Steps 2, 3, and 4 must be executed within a single
  PostgreSQL database transaction to ensure the ledger is never left in a
  partially settled state if the API crashes during the closure.

## 2. Account Dormancy

**Business Context:** To comply with regulatory guidelines (e.g., NRB) and
prevent fraud on abandoned funds, accounts must be frozen after a prolonged
period of inactivity. A dormant account rejects all outgoing customer-initiated
debits (withdrawals, POS swipes, transfers) until the customer completes KYC
verification at a branch.

**The Trigger:** Dormancy is calculated based strictly on the
`last_customer_activity_at` timestamp.

- System-generated postings (Interest Capitalization, Fee Deductions, Tax
  Withholdings) **do not** update this timestamp.
- Customer-initiated mutations (Deposits, Withdrawals, Inward Transfers) **do**
  update this timestamp.

**The State Transition:** An asynchronous nightly background job sweeps the
`accounts` table. If
`Current Date - last_customer_activity_at > Product.dormancy_days`, the account
status is updated from `ACTIVE` to `DORMANT`.

```mermaid
stateDiagram-v2
    [*] --> ACTIVE: Account Opened
    ACTIVE --> ACTIVE: Customer Transaction (Reset Clock)
    ACTIVE --> ACTIVE: System Transaction (Clock Ignored)
    ACTIVE --> DORMANT: Nightly Sweep (Inactivity > 180 days)
    DORMANT --> DORMANT: Customer Transaction (REJECTED)
    DORMANT --> DORMANT: System Transaction (ALLOWED)
    DORMANT --> ACTIVE: Manual KYC Unfreeze by Branch Manager
    DORMANT --> CLOSED: Account Closed
```
