use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use thiserror::Error;
use uuid::Uuid;

pub const GENESIS_HASH: &str = "0000000000000000000000000000000000000000000000000000000000000000";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Direction {
    Debit,
    Credit,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum LedgerError {
    #[error("Imbalanced journal entry")]
    ImbalancedJournalEntry,
    #[error("Insufficient funds")]
    InsufficientFunds,
    #[error("Ledger not found")]
    LedgerNotFound,
    #[error("Zero amount posting")]
    ZeroAmountPosting,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
    #[error("Cryptographic mismatch")]
    CryptographicMismatch,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerState {
    pub id: Uuid,
    pub current_balance: i64,
    pub pending_holds: i64,
    pub overdraft_limit: i64,
    pub version: i32,
    pub previous_state_hash: String,
    pub current_state_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl LedgerState {
    pub fn calculate_hash(&self) -> String {
        let payload = format!(
            "{}{}{}{}",
            self.previous_state_hash,
            self.current_balance,
            self.version,
            self.updated_at.timestamp()
        );
        let mut hasher = Sha256::new();
        hasher.update(payload.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn available_balance(&self) -> Result<i64, LedgerError> {
        self.current_balance
            .checked_sub(self.pending_holds)
            .ok_or(LedgerError::ArithmeticOverflow)?
            .checked_add(self.overdraft_limit)
            .ok_or(LedgerError::ArithmeticOverflow)
    }

    /// Applies a posting to update the current balance.
    fn apply_posting(&mut self, posting: &Posting) -> Result<(), LedgerError> {
        self.current_balance = match posting.direction {
            Direction::Debit => self
                .current_balance
                .checked_sub(posting.amount)
                .ok_or(LedgerError::ArithmeticOverflow)?,
            Direction::Credit => self
                .current_balance
                .checked_add(posting.amount)
                .ok_or(LedgerError::ArithmeticOverflow)?,
        };
        Ok(())
    }

    /// Increments the version for optimistic concurrency control.
    fn increment_version(&mut self) -> Result<(), LedgerError> {
        self.version = self
            .version
            .checked_add(1)
            .ok_or(LedgerError::ArithmeticOverflow)?;
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    pub ledger_id: Uuid,
    pub amount: i64,
    pub direction: Direction,
    pub remark: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: Uuid,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub correlation_id: Uuid,
    pub metadata: Option<serde_json::Value>,
    pub postings: Vec<Posting>,
}

/// Validates that all posting amounts are positive (Invariant 1).
fn validate_positive_amounts(postings: &[Posting]) -> Result<(), LedgerError> {
    if postings.iter().any(|p| p.amount <= 0) {
        return Err(LedgerError::ZeroAmountPosting);
    }
    Ok(())
}

/// Validates that total debits equal total credits (Invariant 2: Double-Entry Balance).
fn validate_double_entry_balance(postings: &[Posting]) -> Result<(), LedgerError> {
    let mut total_debit: i64 = 0;
    let mut total_credit: i64 = 0;

    for p in postings {
        match p.direction {
            Direction::Debit => {
                total_debit = total_debit
                    .checked_add(p.amount)
                    .ok_or(LedgerError::ArithmeticOverflow)?;
            }
            Direction::Credit => {
                total_credit = total_credit
                    .checked_add(p.amount)
                    .ok_or(LedgerError::ArithmeticOverflow)?;
            }
        }
    }

    if total_debit != total_credit {
        return Err(LedgerError::ImbalancedJournalEntry);
    }
    Ok(())
}

/// Validates that all referenced ledgers exist in the states map.
fn validate_ledgers_exist(
    postings: &[Posting],
    states: &HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    if postings.iter().any(|p| !states.contains_key(&p.ledger_id)) {
        return Err(LedgerError::LedgerNotFound);
    }
    Ok(())
}

/// Calculates the net impact (sum of credits minus debits) for each ledger.
fn calculate_net_impacts(postings: &[Posting]) -> Result<HashMap<Uuid, i64>, LedgerError> {
    let mut net_impacts: HashMap<Uuid, i64> = HashMap::new();

    for posting in postings {
        let current_impact = net_impacts.get(&posting.ledger_id).copied().unwrap_or(0);
        let new_impact = match posting.direction {
            Direction::Debit => current_impact
                .checked_sub(posting.amount)
                .ok_or(LedgerError::ArithmeticOverflow)?,
            Direction::Credit => current_impact
                .checked_add(posting.amount)
                .ok_or(LedgerError::ArithmeticOverflow)?,
        };
        net_impacts.insert(posting.ledger_id, new_impact);
    }

    Ok(net_impacts)
}

/// Verifies that all ledgers have sufficient funds after applying the net impacts.
fn verify_sufficient_funds(
    net_impacts: &HashMap<Uuid, i64>,
    states: &HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    for (ledger_id, impact) in net_impacts {
        // Safe to use expect here since we've already validated ledger existence
        let state = states.get(ledger_id).ok_or(LedgerError::LedgerNotFound)?;

        let available = state.available_balance()?;

        if available
            .checked_add(*impact)
            .ok_or(LedgerError::ArithmeticOverflow)?
            < 0
        {
            return Err(LedgerError::InsufficientFunds);
        }
    }
    Ok(())
}

/// Applies all postings to update ledger balances.
fn apply_postings(
    postings: &[Posting],
    states: &mut HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    for posting in postings {
        let state = states
            .get_mut(&posting.ledger_id)
            .ok_or(LedgerError::LedgerNotFound)?;
        state.apply_posting(posting)?;
    }
    Ok(())
}

/// Increments version for all mutated accounts (Optimistic Concurrency Control).
fn increment_versions(
    postings: &[Posting],
    states: &mut HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    let mutated_accounts: HashSet<Uuid> = postings.iter().map(|p| p.ledger_id).collect();

    for account_id in mutated_accounts {
        let state = states
            .get_mut(&account_id)
            .ok_or(LedgerError::LedgerNotFound)?;
        state.increment_version()?;
    }
    Ok(())
}

/// Updates the updated_at timestamp for mutated accounts
fn update_timestamps(
    postings: &[Posting],
    states: &mut HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    let now = Utc::now();
    let mutated_accounts: HashSet<Uuid> = postings.iter().map(|p| p.ledger_id).collect();

    for account_id in mutated_accounts {
        let state = states
            .get_mut(&account_id)
            .ok_or(LedgerError::LedgerNotFound)?;
        state.updated_at = now;
    }
    Ok(())
}

/// Verifies cryptographic hashes for all mutated accounts before processing.
fn verify_cryptographic_hashes(
    postings: &[Posting],
    states: &HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    let mutated_accounts: HashSet<Uuid> = postings.iter().map(|p| p.ledger_id).collect();

    for account_id in mutated_accounts {
        let state = states.get(&account_id).ok_or(LedgerError::LedgerNotFound)?;
        if state.calculate_hash() != state.current_state_hash {
            return Err(LedgerError::CryptographicMismatch);
        }
    }
    Ok(())
}

/// Updates cryptographic hashes for mutated accounts (Hash Chaining).
fn update_hashes(
    postings: &[Posting],
    states: &mut HashMap<Uuid, LedgerState>,
) -> Result<(), LedgerError> {
    let mutated_accounts: HashSet<Uuid> = postings.iter().map(|p| p.ledger_id).collect();

    for account_id in mutated_accounts {
        let state = states
            .get_mut(&account_id)
            .ok_or(LedgerError::LedgerNotFound)?;
        state.previous_state_hash = state.current_state_hash.clone();
        state.current_state_hash = state.calculate_hash();
    }
    Ok(())
}

/// Applies a journal entry to the ledger states, enforcing all invariants and constraints.
///
/// # Invariants Enforced
/// - Invariant 1: All posting amounts must be positive
/// - Invariant 2: Total debits must equal total credits (double-entry balance)
///
/// # Preconditions
/// - All referenced ledgers must exist in the states map
///
/// # Postconditions
/// - Limit enforcement: Available balance must remain non-negative after applying impacts
/// - Optimistic concurrency: Version is incremented for each mutated account
pub fn apply_journal_entry(
    entry: &JournalEntry,
    mut states: HashMap<Uuid, LedgerState>,
) -> Result<HashMap<Uuid, LedgerState>, LedgerError> {
    // Validate invariants
    validate_positive_amounts(&entry.postings)?;
    validate_double_entry_balance(&entry.postings)?;

    // Validate preconditions
    validate_ledgers_exist(&entry.postings, &states)?;
    verify_cryptographic_hashes(&entry.postings, &states)?;

    // Calculate and verify impacts before mutation
    let net_impacts = calculate_net_impacts(&entry.postings)?;
    verify_sufficient_funds(&net_impacts, &states)?;

    // Apply mutations
    apply_postings(&entry.postings, &mut states)?;
    increment_versions(&entry.postings, &mut states)?;
    update_timestamps(&entry.postings, &mut states)?;
    update_hashes(&entry.postings, &mut states)?;

    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    // Helper functions for testing
    fn valid_ledger_state(id: Uuid) -> LedgerState {
        let mut state = LedgerState {
            id,
            current_balance: 1000,
            pending_holds: 0,
            overdraft_limit: 0,
            version: 1,
            previous_state_hash: GENESIS_HASH.to_string(),
            current_state_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        state.current_state_hash = state.calculate_hash();
        state
    }

    #[test]
    fn test_apply_journal_entry_fails_with_zero_amount() {
        let ledger_id = Uuid::new_v4();
        let posting = Posting {
            ledger_id,
            amount: 0,
            direction: Direction::Credit,
            remark: None,
            created_at: Utc::now(),
        };
        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings: vec![posting],
        };

        let mut states = HashMap::new();
        states.insert(ledger_id, valid_ledger_state(ledger_id));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::ZeroAmountPosting);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_negative_amount() {
        let ledger_id = Uuid::new_v4();
        let posting = Posting {
            ledger_id,
            amount: -100,
            direction: Direction::Credit,
            remark: None,
            created_at: Utc::now(),
        };
        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings: vec![posting],
        };

        let mut states = HashMap::new();
        states.insert(ledger_id, valid_ledger_state(ledger_id));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::ZeroAmountPosting);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_imbalanced_entry() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 50,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let mut states = HashMap::new();
        states.insert(ledger_id_1, valid_ledger_state(ledger_id_1));
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::ImbalancedJournalEntry);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_ledger_not_found() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let mut states = HashMap::new();
        states.insert(ledger_id_1, valid_ledger_state(ledger_id_1));
        // Missing ledger_id_2

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::LedgerNotFound);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_insufficient_funds() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 2000,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 2000,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let mut states = HashMap::new();
        states.insert(ledger_id_1, valid_ledger_state(ledger_id_1));
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::InsufficientFunds);
    }

    #[test]
    fn test_apply_journal_entry_success() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let mut states = HashMap::new();
        states.insert(ledger_id_1, valid_ledger_state(ledger_id_1));
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        let result = apply_journal_entry(&journal_entry, states.clone());

        assert!(result.is_ok());

        let new_states = result.unwrap();

        let new_state_1 = new_states.get(&ledger_id_1).unwrap();
        assert_eq!(new_state_1.current_balance, 900);
        assert_eq!(new_state_1.version, 2);
        assert_eq!(
            new_state_1.previous_state_hash,
            states.get(&ledger_id_1).unwrap().current_state_hash
        );
        assert_eq!(new_state_1.current_state_hash, new_state_1.calculate_hash());

        let new_state_2 = new_states.get(&ledger_id_2).unwrap();
        assert_eq!(new_state_2.current_balance, 1100);
        assert_eq!(new_state_2.version, 2);
        assert_eq!(
            new_state_2.previous_state_hash,
            states.get(&ledger_id_2).unwrap().current_state_hash
        );
        assert_eq!(new_state_2.current_state_hash, new_state_2.calculate_hash());
    }

    proptest! {
        #[test]
        fn property_test_balance_integrity(
            initial_balance in 100..10_000i64,
            transfer_amount in 1..100i64
        ) {
             let ledger_id_1 = Uuid::new_v4();
             let ledger_id_2 = Uuid::new_v4();

             let postings = vec![
                Posting {
                    ledger_id: ledger_id_1,
                    amount: transfer_amount,
                    direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
                Posting {
                    ledger_id: ledger_id_2,
                    amount: transfer_amount,
                    direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
            ];

            let journal_entry = JournalEntry {
                id: Uuid::new_v4(),
                description: "Proptest Entry".to_string(),
                timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
                postings,
            };

            let mut states = HashMap::new();

            let mut state_1 = valid_ledger_state(ledger_id_1);
            state_1.current_balance = initial_balance;
            state_1.current_state_hash = state_1.calculate_hash();
            states.insert(ledger_id_1, state_1);

            let mut state_2 = valid_ledger_state(ledger_id_2);
            state_2.current_balance = initial_balance;
            state_2.current_state_hash = state_2.calculate_hash();
            states.insert(ledger_id_2, state_2);

            let result = apply_journal_entry(&journal_entry, states.clone());
            prop_assert!(result.is_ok());

            let new_states = result.unwrap();
            let new_state_1 = new_states.get(&ledger_id_1).unwrap();
            let new_state_2 = new_states.get(&ledger_id_2).unwrap();

            // Property: Total funds should remain constant
            prop_assert_eq!(
                new_state_1.current_balance + new_state_2.current_balance,
                initial_balance + initial_balance
            );
        }
    }

    #[test]
    fn test_apply_journal_entry_respects_overdraft_and_holds() {
        let ledger_id_1 = Uuid::new_v4(); // Sender
        let ledger_id_2 = Uuid::new_v4(); // Receiver

        let mut state_1 = valid_ledger_state(ledger_id_1);
        // Current: 1000. Holds: 400. Limit: 500.
        // Available = 1000 - 400 + 500 = 1100.
        state_1.current_balance = 1000;
        state_1.pending_holds = 400;
        state_1.overdraft_limit = 500;

        let mut states = HashMap::new();
        states.insert(ledger_id_1, state_1);
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        // Attempt to debit 1200 (Exceeds available of 1100) -> Should Fail
        let fail_postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 1200,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 1200,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];
        let fail_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Fail".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings: fail_postings,
        };

        let fail_result = apply_journal_entry(&fail_entry, states.clone());
        assert_eq!(fail_result.unwrap_err(), LedgerError::InsufficientFunds);

        // Attempt to debit 1100 (Exactly drains available) -> Should Pass, leaving balance at -100
        let pass_postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 1100,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 1100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];
        let pass_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Pass".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings: pass_postings,
        };

        let pass_result = apply_journal_entry(&pass_entry, states);
        assert!(pass_result.is_ok());
        assert_eq!(
            pass_result
                .unwrap()
                .get(&ledger_id_1)
                .unwrap()
                .current_balance,
            -100
        );
    }

    #[test]
    fn test_apply_journal_entry_evaluates_net_impact_atomically() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let mut state_1 = valid_ledger_state(ledger_id_1);
        state_1.current_balance = 1000; // Available is 1000

        let mut states = HashMap::new();
        states.insert(ledger_id_1, state_1);
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        // Two debits of 600 against the same account. Total debit = 1200. Should fail.
        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 600,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_1,
                amount: 600,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 1200,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Net Impact Test".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let result = apply_journal_entry(&journal_entry, states);
        assert_eq!(result.unwrap_err(), LedgerError::InsufficientFunds);
    }

    #[test]
    fn test_ledger_state_available_balance_calculation() {
        let state = LedgerState {
            id: Uuid::new_v4(),
            current_balance: 1000,
            pending_holds: 400,
            overdraft_limit: 500,
            version: 1,
            previous_state_hash: String::new(),
            current_state_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        // 1000 - 400 + 500 = 1100
        assert_eq!(state.available_balance().unwrap(), 1100);

        let state_negative = LedgerState {
            id: Uuid::new_v4(),
            current_balance: -200, // Deep in overdraft
            pending_holds: 100,
            overdraft_limit: 500,
            version: 1,
            previous_state_hash: String::new(),
            current_state_hash: String::new(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        // -200 - 100 + 500 = 200
        assert_eq!(state_negative.available_balance().unwrap(), 200);
    }

    #[test]
    fn test_apply_journal_entry_arithmetic_overflow() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        // 1. Trigger ArithmeticOverflow in available_balance()
        let mut state_1 = valid_ledger_state(account_id_1);
        state_1.current_balance = i64::MAX;
        state_1.overdraft_limit = 100; // i64::MAX + 100 -> Overflow
        state_1.current_state_hash = state_1.calculate_hash();

        let mut states = HashMap::new();
        states.insert(account_id_1, state_1);
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        let postings = vec![
            Posting {
                ledger_id: account_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: account_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Overflow Test".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let result = apply_journal_entry(&journal_entry, states);
        assert_eq!(result.unwrap_err(), LedgerError::ArithmeticOverflow);
    }

    #[test]
    fn test_apply_journal_entry_version_overflow() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let mut state_1 = valid_ledger_state(ledger_id_1);
        // Set version to maximum possible i32 to trigger overflow on increment
        state_1.version = i32::MAX;
        state_1.current_state_hash = state_1.calculate_hash();

        let mut states = HashMap::new();
        states.insert(ledger_id_1, state_1);
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Version Overflow Test".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let result = apply_journal_entry(&journal_entry, states);
        assert_eq!(result.unwrap_err(), LedgerError::ArithmeticOverflow);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_cryptographic_mismatch() {
        let ledger_id_1 = Uuid::new_v4();
        let ledger_id_2 = Uuid::new_v4();

        let mut state_1 = valid_ledger_state(ledger_id_1);
        // Tamper with the balance without updating the hash
        state_1.current_balance = 2000;

        let mut states = HashMap::new();
        states.insert(ledger_id_1, state_1);
        states.insert(ledger_id_2, valid_ledger_state(ledger_id_2));

        let postings = vec![
            Posting {
                ledger_id: ledger_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: ledger_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Tamper Test".to_string(),
            timestamp: Utc::now(),
            created_at: Utc::now(),
            correlation_id: Uuid::new_v4(),
            metadata: None,
            postings,
        };

        let result = apply_journal_entry(&journal_entry, states);
        assert_eq!(result.unwrap_err(), LedgerError::CryptographicMismatch);
    }

    #[test]
    fn test_apply_posting_debit_overflow() {
        let mut state = valid_ledger_state(Uuid::new_v4());
        state.current_balance = i64::MIN;
        let posting = Posting {
            ledger_id: state.id,
            amount: 1, // i64::MIN - 1 -> Overflow
            direction: Direction::Debit,
            remark: None,
            created_at: Utc::now(),
        };
        assert_eq!(
            state.apply_posting(&posting),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_apply_posting_credit_overflow() {
        let mut state = valid_ledger_state(Uuid::new_v4());
        state.current_balance = i64::MAX;
        let posting = Posting {
            ledger_id: state.id,
            amount: 1, // i64::MAX + 1 -> Overflow
            direction: Direction::Credit,
            remark: None,
            created_at: Utc::now(),
        };
        assert_eq!(
            state.apply_posting(&posting),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_validate_double_entry_balance_debit_overflow() {
        let postings = vec![
            Posting {
                ledger_id: Uuid::new_v4(),
                amount: i64::MAX,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: Uuid::new_v4(),
                amount: 1,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
        ];
        assert_eq!(
            validate_double_entry_balance(&postings),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_validate_double_entry_balance_credit_overflow() {
        let postings = vec![
            Posting {
                ledger_id: Uuid::new_v4(),
                amount: i64::MAX,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id: Uuid::new_v4(),
                amount: 1,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];
        assert_eq!(
            validate_double_entry_balance(&postings),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_calculate_net_impacts_debit_overflow() {
        let ledger_id = Uuid::new_v4();
        let postings = vec![
            Posting {
                ledger_id,
                amount: i64::MAX,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id,
                amount: 2,
                direction: Direction::Debit,
                remark: None,
                created_at: Utc::now(),
            },
        ];
        assert_eq!(
            calculate_net_impacts(&postings),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_calculate_net_impacts_credit_overflow() {
        let ledger_id = Uuid::new_v4();
        let postings = vec![
            Posting {
                ledger_id,
                amount: i64::MAX,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
            Posting {
                ledger_id,
                amount: 1,
                direction: Direction::Credit,
                remark: None,
                created_at: Utc::now(),
            },
        ];
        assert_eq!(
            calculate_net_impacts(&postings),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_verify_sufficient_funds_overflow() {
        let ledger_id = Uuid::new_v4();
        let mut state = valid_ledger_state(ledger_id);
        state.current_balance = i64::MAX;

        let mut states = HashMap::new();
        states.insert(ledger_id, state);

        let mut net_impacts = HashMap::new();
        net_impacts.insert(ledger_id, 1);

        assert_eq!(
            verify_sufficient_funds(&net_impacts, &states),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_apply_postings_ledger_not_found() {
        let postings = vec![Posting {
            ledger_id: Uuid::new_v4(),
            amount: 100,
            direction: Direction::Debit,
            remark: None,
            created_at: Utc::now(),
        }];
        let mut states = HashMap::new();
        assert_eq!(
            apply_postings(&postings, &mut states),
            Err(LedgerError::LedgerNotFound)
        );
    }

    #[test]
    fn test_apply_postings_arithmetic_overflow() {
        let ledger_id = Uuid::new_v4();
        let mut state = valid_ledger_state(ledger_id);
        state.current_balance = i64::MAX;

        let mut states = HashMap::new();
        states.insert(ledger_id, state);

        let postings = vec![Posting {
            ledger_id,
            amount: 1, // i64::MAX + 1 -> Overflow
            direction: Direction::Credit,
            remark: None,
            created_at: Utc::now(),
        }];
        assert_eq!(
            apply_postings(&postings, &mut states),
            Err(LedgerError::ArithmeticOverflow)
        );
    }

    #[test]
    fn test_increment_versions_ledger_not_found() {
        let postings = vec![Posting {
            ledger_id: Uuid::new_v4(),
            amount: 100,
            direction: Direction::Debit,
            remark: None,
            created_at: Utc::now(),
        }];
        let mut states = HashMap::new();
        assert_eq!(
            increment_versions(&postings, &mut states),
            Err(LedgerError::LedgerNotFound)
        );
    }

    #[test]
    fn test_update_hashes_ledger_not_found() {
        let postings = vec![Posting {
            ledger_id: Uuid::new_v4(),
            amount: 100,
            direction: Direction::Debit,
            remark: None,
            created_at: Utc::now(),
        }];
        let mut states = HashMap::new();
        assert_eq!(
            update_hashes(&postings, &mut states),
            Err(LedgerError::LedgerNotFound)
        );
    }

    #[test]
    fn test_verify_cryptographic_hashes_ledger_not_found() {
        let postings = vec![Posting {
            ledger_id: Uuid::new_v4(),
            amount: 100,
            direction: Direction::Debit,
            remark: None,
            created_at: Utc::now(),
        }];
        let states = HashMap::new();
        assert_eq!(
            verify_cryptographic_hashes(&postings, &states),
            Err(LedgerError::LedgerNotFound)
        );
    }
}
