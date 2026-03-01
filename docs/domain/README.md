# Domain & Business Workflows

This folder contains the core business logic, ledger mechanics, and product
lifecycle workflows for the Keva Core Banking System.

As a double-entry ledger optimized for maximum throughput, the mathematical
operations here are completely abstracted from the API presentation layer. These
documents serve as the single source of truth for how account transitions,
accruals, and end-of-day processes literally mutate the state of the database.

## Contents

- [Account Lifecycle (Closure & Settlement)](./account-lifecycle.md)
  - Details the strict real-time settlement required when an account is closed
    before a scheduled batch payout.
- [End-of-Day (EOD) & Batch Processing](./end-of-day-processing.md)
  - Defines the mathematical distinction between daily Interest Accrual
    (liability recognition) and periodic Capitalization (customer payout), as
    well as the 24/7 rolling cut-off rules.
