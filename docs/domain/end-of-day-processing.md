# End-of-Day (EOD) & Batch Processing

This document defines the business logic and ledger workflows for asynchronous End-of-Day (EOD) operations, specifically focusing on continuous 24/7 uptime and Interest Accrual vs. Capitalization.

## 1. The Rolling Cut-Off Time
Keva operates as a 24/7 system. The business day strictly ends at `23:59:59`.
* **Invariant:** Batch jobs (even if executed at 02:00 AM) must calculate balances using strictly immutable ledger queries filtered by the `23:59:59` timestamp. Real-time transactions occurring after midnight must never bleed into the previous day's interest calculations.

## 2. Interest Accrual (Daily)
Interest accrual is the daily recognition of the bank's liability to the customer. It is strictly an internal ledger movement and does not increase the customer's spendable balance.

* **Trigger:** Daily automated batch job.
* **Accounting Movement:**
  * **Debit:** `Interest Expense GL` (Bank P&L)
  * **Credit:** `Accrued Interest Payable GL` (Liability, tagged with the customer's `account_id`)

## 3. Interest Capitalization (Periodic Payout)
Capitalization is the actual settlement of accrued interest into the customer's available balance. The frequency (Monthly, Quarterly, Annually) is dictated by the specific Product Catalog rules assigned to the account.

* **Trigger:** End-of-Month or End-of-Quarter batch job, OR a manual Account Closure event.
* **Accounting Movement:**
  * **Debit:** `Accrued Interest Payable GL` (Zeroing out the liability)
  * **Credit:** `Customer Savings Account` (Increasing spendable balance)

  