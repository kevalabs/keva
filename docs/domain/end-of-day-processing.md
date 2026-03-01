# End-of-Day (EOD) & Batch Processing

This document defines the business logic and ledger workflows for asynchronous
End-of-Day (EOD) operations, specifically focusing on continuous 24/7 uptime and
Interest Accrual vs. Capitalization.

## 1. The Rolling Cut-Off Time

Keva operates as a 24/7 system. The business day strictly ends at `23:59:59`.

- **Invariant:** Batch jobs (even if executed at 02:00 AM) must calculate
  balances using strictly immutable ledger queries filtered by the `23:59:59`
  timestamp. Real-time transactions occurring after midnight must never bleed
  into the previous day's interest calculations.

## 2. Interest Accrual (Daily)

Interest accrual is the daily recognition of the bank's liability to the
customer. It is strictly an internal ledger movement and does not increase the
customer's spendable balance.

- **Trigger:** Daily automated batch job.
- **Accounting Movement:**
  - **Debit:** `Interest Expense GL` (Bank P&L)
  - **Credit:** `Accrued Interest Payable GL` (Liability, tagged with the
    customer's `account_id`)

## 3. Interest Capitalization (Periodic Payout)

Capitalization is the actual settlement of accrued interest into the customer's
available balance. The frequency (Monthly, Quarterly, Annually) is dictated by
the specific Product Catalog rules assigned to the account.

- **Trigger:** End-of-Month or End-of-Quarter batch job, OR a manual Account
  Closure event.
- **Accounting Movement:**
  - **Debit:** `Accrued Interest Payable GL` (Zeroing out the liability)
  - **Credit:** `Customer Savings Account` (Increasing spendable balance)

## 4. EOD Processing & Dormant Accounts

**Business Context:** Per regulatory guidelines, funds held in dormant savings
accounts must continue to accrue interest exactly like active accounts. The bank
retains the liability for the funds regardless of the customer's activity
status.

**Operational Priority:**
To optimize the customer experience for active users, the End-of-Day batch
processing engine implements a strict priority queue.

- **Priority 1:** All `ACTIVE` accounts are batched and processed first. This
  ensures active users see updated balances and accrued interest early in the
  morning.
- **Priority 2:** All `DORMANT` accounts are batched and processed only after
  the active queue is entirely exhausted.

## 5. Hold / Authorization Expiration Sweep

**Business Context:** When a merchant places a pre-authorization hold on an
account, it reduces the Available Balance without altering the Current Balance.
If a merchant fails to capture the hold (e.g., a cancelled hotel reservation),
the system must automatically release the funds to prevent permanent customer
lock-up.

**Operational Flow:**
Every hold record carries an `expires_at` timestamp dictated by network rules or
the Keva product catalog.

- **The Trigger:** A nightly End-of-Day background worker executes an expiration
  sweep.
- **The Action:** The worker deletes (or marks as expired) any record in the
  `account_holds` table where the expiration timestamp has passed.
- **The Result:** The customer's Available Balance instantly and silently
  increases without requiring any modifications to the core `accounts` ledger or
  generating accounting journal entries.
