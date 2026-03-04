# Keva Domain Glossary & Ubiquitous Language

This glossary defines the strict domain vocabulary used across the Keva core
banking system. Engineers, product managers, and architects must use these exact
terms to prevent systemic accounting or logic errors.

## 1. Balances & Funds

- **Current Balance (Ledger Balance):** The absolute, settled mathematical truth
  of an account at any given microsecond. It is the strict sum of all historical
  `postings`. It does not fluctuate based on pending authorizations.
- **Available Balance (Spendable Balance):** The dynamic, immediate purchasing
  power of the user. Calculated as:
  `Current Balance - Pending Holds + Overdraft Limit`. Used exclusively for
  transaction validation (e.g., POS terminal authorizations).
- **Hold (Authorization):** A temporary reduction in the Available Balance
  placed by a merchant. It is physically isolated in a separate `account_holds`
  table to prevent Optimistic Concurrency Control (OCC) conflicts on the core
  ledger. Holds automatically expire if not captured.
- **Overdraft Limit:** A pre-approved line of credit attached directly to a
  deposit account, allowing the Current Balance to drop below zero. Evaluated
  strictly against the Available Balance formula.

## 2. Ledger & Time Mechanics

- **Double-Entry Accounting:** The foundational mathematical constraint where
  every transaction must balance to zero (`Sum(Debits) - Sum(Credits) == 0`).
- **Bitemporal Immutability:** The concept applied to the Product Catalog where
  records are never updated or deleted. State changes are tracked using two time
  axes (`effective_from` and `effective_to`), allowing the system to
  retroactively understand what the rules *were* at any point in the past.
- **Rolling Cut-Off Time:** The strict, 24/7 time boundary (e.g., `23:59:59`).
  End-of-Day (EOD) batch jobs calculate daily values using immutable ledger
  queries strictly filtered by this timestamp, ensuring night-shift transactions
  do not bleed into the previous day's math.
- **Interest Accrual:** The daily, mathematical recognition of liability (or
  receivable debt). It is an internal General Ledger (GL) movement and does not
  alter the customer's account balance.
- **Interest Capitalization (Settlement):** The actual payout (or deduction) of
  accrued interest into the customer's account balance. Occurs periodically
  (e.g., Monthly) or in real-time during an Account Closure.

## 3. Product vs. Account Constraints

- **Product (The Blueprint):** The bitemporal configuration living in
  `keva-catalog`. It defines the boundaries, fees, and standard interest rates
  for a class of accounts (e.g., "Standard Savings").
- **Arrangement / Account (The Instance):** The specific instantiation of a
  Product for a specific customer. Contains the user's current state, balances,
  and specific underwriting decisions (like an individual Overdraft Limit).
- **Account-Level Override:** A temporal, negotiated contract (e.g., a VIP
  client receiving 12% interest instead of the standard 10%). Stored directly on
  the Account instance with an `expires_at` date to prevent polluting the
  Product Catalog with single-use clones.

## 4. Account Lifecycle States

- **Active:** Normal operating state. All legitimate transactions are permitted.
- **Dormant:** A frozen state triggered by prolonged *customer-initiated*
  inactivity. Rejects outward customer transfers but continues to accept system
  deposits and accrue interest. Processed as Priority 2 in the EOD batch engine.
- **Closed:** A terminal state. Mathematically requires both the Current Balance
  and all linked Accrued Liabilities to be exactly `0.00`.

## 5. Concurrency & Operations

- **Optimistic Concurrency Control (OCC):** The database locking strategy
  relying on a `version` integer on the `accounts` table. Rejects transactions
  if the underlying state was modified by another thread during processing,
  preventing microsecond double-spending.
- **Virtual Clearing Account:** A temporary, internal routing account used to
  intercept and process high-density batch files (like corporate payroll). It
  eliminates database row-lock contention on the corporate source account by
  allowing background workers to process individual credits in parallel.
- **Idempotency Key:** A unique, client-generated string used to safely retry
  network requests without causing double-charges. Enforced via a strict 24-hour
  sliding window expiration at the API layer.
- **Claim Check Pattern:** An operational workflow where the API accepts a
  request (e.g., a large file upload or an impatient user's double-click),
  instantly returns a tracking ID, and processes the heavy ledger math
  asynchronously.
