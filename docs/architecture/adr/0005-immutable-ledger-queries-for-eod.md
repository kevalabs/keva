# ADR 0005: Immutable Ledger Queries for EOD Balance Calculation

**Status:** Accepted  
**Date:** 2026-03-01  

## 1. Context and Problem Statement
Keva operates as a 24/7 core banking system. End-of-Day (EOD) batch processing (e.g., daily interest accrual) must calculate values based on the exact account balance at the daily cut-off time (e.g., 23:59:59). 

Because the API remains live, the `accounts.balance` column will continue to mutate with new transactions immediately after midnight. We need a reliable mechanism to determine the exact cut-off balance without halting the system or serving inaccurate data.

## 2. Considered Options
* **Option 1: The Midnight Snapshot (Lock & Copy).** At exactly 23:59:59, a cron job copies all balances from the `accounts` table to a `daily_snapshots` table. (Rejected: Causes massive I/O spikes and potential API locks at midnight).
* **Option 2: The Immutable Ledger Query (Pure Compute).** EOD workers dynamically calculate the cut-off balance by summing the immutable double-entry `postings` table up to the exact cut-off microsecond: `SELECT SUM(amount) FROM postings WHERE account_id = X AND created_at <= 'Cut-Off Time'`.
* **Option 3: The Hybrid Rolling Snapshot.** A background job asynchronously calculates yesterday's delta and appends it to a snapshot table. (Rejected: Reintroduces stateful job dependencies and synchronization risks).

## 3. Decision Outcome
We are proceeding with **Option 2: The Immutable Ledger Query**. 



For all EOD batch processes, Tokio workers will completely ignore the dynamically updating `accounts.balance` column. Instead, they will query the `postings` table, utilizing the `created_at` timestamp to filter out any transactions that occurred after the cut-off boundary. 

## 4. Rationale
* **Single Source of Truth:** The `postings` table is mathematically pure. We eliminate the risk of a snapshot table drifting out of sync with the actual ledger.
* **Zero Disruption:** The 24/7 API experiences zero locking or I/O degradation when the day rolls over. Night-shift transactions continue entirely unaffected.
* **Bitemporal Safety:** If a batch job crashes and has to be re-run three days later, it will still calculate the exact same balance, because the query is locked to the specific historical timestamp, not the current state.

## 5. Consequences
* **Positive:** Massive reduction in moving parts. No snapshot cron jobs to monitor or maintain.
* **Negative/Mitigation:** As the database grows, running `SUM()` over years of history for millions of accounts will consume significant CPU. 
* **Scaling Strategy:** To mitigate the compute load, Keva will rely on **PostgreSQL Native Table Partitioning**. The `postings` table will be partitioned by month (e.g., `postings_2026_02`). The database engine will automatically prune partitions during the query, ensuring the `SUM()` operation remains highly performant even at massive scale.