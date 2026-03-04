---
description: Triggers when discussing ledger math, OCC, balances, holds, overdrafts, or double-entry accounting constraints.
---

# Keva Core Ledger Physics

When designing schemas or writing SQL/Rust logic for the ledger, apply these exact mechanics:

1. **Available Balance Calculation:** `Available Balance = Current Balance - Pending Holds + Overdraft Limit`.
2. **Holds Isolation:** Pending authorizations MUST be stored in the `account_holds` table, never as a numeric column on the `accounts` table.
3. **OCC Updates:** Ledger mutations must execute: `UPDATE accounts SET balance = new_balance, version = version + 1 WHERE id = account_id AND version = current_version`.
4. **EOD Queries:** End-of-Day balance calculations must NOT read the `accounts.balance` column. They must use immutable ledger queries against the `postings` table filtered by the cut-off timestamp.