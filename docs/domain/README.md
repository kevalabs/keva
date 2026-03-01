# Domain & Business Workflows

This folder contains the core business logic, ledger mechanics, and product
lifecycle workflows for the Keva Core Banking System.

## Contents

- [Account Lifecycle (Closure & Settlement)](./account-lifecycle.md)
  - Details the strict real-time settlement required when an account is closed
    before a scheduled batch payout.

- [End-of-Day (EOD) & Batch Processing](./end-of-day-processing.md)
  - Defines the mathematical distinction between daily Interest Accrual
    (liability recognition) and periodic Capitalization (customer payout), as
    well as the 24/7 rolling cut-off rules.

- [Balance Mechanics & Ledger Math](./balance-mechanics.md)
  - Defines the distinct realities of Current Balance vs. Available Balance
    and standard hold/authorization mechanics.
