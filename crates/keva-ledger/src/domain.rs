use chrono::{DateTime, Utc};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug, PartialEq)]
pub enum LedgerError {
    #[error("Double-entry math violation: Debits do not equal Credits.")]
    ImbalancedJournalEntry,
    #[error("Optimistic Concurrency Control (OCC) mismatch. State mutated by another thread.")]
    VersionConflict,
    #[error("Transaction exceeds available balance and overdraft limits.")]
    InsufficientFunds,
    #[error("Ledger not found {0}")]
    LedgerNotFound(Uuid),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Debit,
    Credit,
}

#[derive(Debug, Clone)]
pub struct LedgerState {
    pub current_balance: i64,
    pub id: Uuid,
    pub overdraft_limit: i64,
    pub version: i32,
}

impl LedgerState {
    pub fn available_balance(&self, pending_holds: i64) -> i64 {
        self.current_balance - pending_holds + self.overdraft_limit
    }
}

/// A single leg of a transaction.
#[derive(Debug, Clone)]
pub struct Posting {
    pub account_id: Uuid,
    pub amount: i64,
    pub direction: Direction,
    /// Granular narration for customer bank statements.
    pub remark: Option<String>,
}

/// The parent transaction that guarantees the Double-Entry rule.
#[derive(Debug, Clone)]
pub struct JournalEntry {
    pub id: Uuid,
    /// Global narrative for internal bank auditing.
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub postings: Vec<Posting>,
}

pub fn apply_journal_entry(
    entry: &JournalEntry,
    mut accounts: Vec<LedgerState>,
) -> Result<Vec<LedgerState>, LedgerError> {
    let mut debit_sum: i64 = 0;
    let mut credit_sum: i64 = 0;

    for posting in &entry.postings {
        match posting.direction {
            Direction::Debit => debit_sum += posting.amount,
            Direction::Credit => credit_sum += posting.amount,
        }
    }

    if debit_sum != credit_sum {
        return Err(LedgerError::ImbalancedJournalEntry);
    }

    for posting in &entry.postings {
        let account = accounts
            .iter_mut()
            .find(|a| a.id == posting.account_id)
            .ok_or(LedgerError::LedgerNotFound(posting.account_id))?;

        match posting.direction {
            Direction::Debit => {
                account.current_balance -= posting.amount;
            }
            Direction::Credit => {
                account.current_balance += posting.amount;
            }
        }

        account.version += 1;
    }

    Ok(accounts)
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
