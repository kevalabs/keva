use chrono::{DateTime, Utc};
use std::collections::HashMap;
use thiserror::Error;
use uuid::Uuid;

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
    #[error("Account not found")]
    AccountNotFound,
    #[error("Zero amount posting")]
    ZeroAmountPosting,
    #[error("Arithmetic overflow")]
    ArithmeticOverflow,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LedgerState {
    pub id: Uuid,
    pub current_balance: i64,
    pub pending_holds: i64,
    pub overdraft_limit: i64,
    pub version: i32,
}

impl LedgerState {
    pub fn available_balance(&self) -> Result<i64, LedgerError> {
        self.current_balance
            .checked_sub(self.pending_holds)
            .ok_or(LedgerError::ArithmeticOverflow)?
            .checked_add(self.overdraft_limit)
            .ok_or(LedgerError::ArithmeticOverflow)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posting {
    pub account_id: Uuid,
    pub amount: i64,
    pub direction: Direction,
    pub remark: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JournalEntry {
    pub id: Uuid,
    pub description: String,
    pub timestamp: DateTime<Utc>,
    pub postings: Vec<Posting>,
}

pub fn apply_journal_entry(
    entry: &JournalEntry,
    mut states: HashMap<Uuid, LedgerState>,
) -> Result<HashMap<Uuid, LedgerState>, LedgerError> {
    // Invariant 1: Positive Amounts
    if entry.postings.iter().any(|p| p.amount <= 0) {
        return Err(LedgerError::ZeroAmountPosting);
    }

    // Invariant 2: Double-Entry Balance
    let mut total_debit: i64 = 0;
    let mut total_credit: i64 = 0;

    for p in &entry.postings {
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

    // Precondition 2: Accounts Must Exist
    // Ensure all accounts referenced in postings exist in states
    if entry
        .postings
        .iter()
        .any(|p| !states.contains_key(&p.account_id))
    {
        return Err(LedgerError::AccountNotFound);
    }

    // Apply net mutations to verify Precondition 3 / Postcondition 2 (Limits) early
    let mut net_impacts: HashMap<Uuid, i64> = HashMap::new();
    for posting in &entry.postings {
        let current_impact = net_impacts.get(&posting.account_id).copied().unwrap_or(0);
        let new_impact = match posting.direction {
            Direction::Debit => current_impact
                .checked_sub(posting.amount)
                .ok_or(LedgerError::ArithmeticOverflow)?,
            Direction::Credit => current_impact
                .checked_add(posting.amount)
                .ok_or(LedgerError::ArithmeticOverflow)?,
        };
        net_impacts.insert(posting.account_id, new_impact);
    }

    // Verify limit enforcement before actual mutation
    for (account_id, impact) in net_impacts {
        let state = states.get(&account_id).unwrap();
        // Postcondition 2: Limit Enforcement using available_balance()
        let available = state.available_balance()?;

        if available
            .checked_add(impact)
            .ok_or(LedgerError::ArithmeticOverflow)?
            < 0
        {
            return Err(LedgerError::InsufficientFunds);
        }
    }

    // Once all checks pass, apply mutations and increment version
    for posting in &entry.postings {
        let state = states.get_mut(&posting.account_id).unwrap();
        match posting.direction {
            Direction::Debit => {
                state.current_balance = state
                    .current_balance
                    .checked_sub(posting.amount)
                    .ok_or(LedgerError::ArithmeticOverflow)?;
            }
            Direction::Credit => {
                state.current_balance = state
                    .current_balance
                    .checked_add(posting.amount)
                    .ok_or(LedgerError::ArithmeticOverflow)?;
            }
        }
    }

    // Postcondition 3: Optimistic Concurrency - only increment version once per account mutated
    let mutated_accounts: std::collections::HashSet<Uuid> =
        entry.postings.iter().map(|p| p.account_id).collect();
    for account_id in mutated_accounts {
        let state = states.get_mut(&account_id).unwrap();
        state.version = state
            .version
            .checked_add(1)
            .ok_or(LedgerError::ArithmeticOverflow)?;
    }

    Ok(states)
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use std::collections::HashMap;

    // Helper functions for testing
    fn valid_ledger_state(id: Uuid) -> LedgerState {
        LedgerState {
            id,
            current_balance: 1000,
            pending_holds: 0,
            overdraft_limit: 0,
            version: 1,
        }
    }

    #[test]
    fn test_apply_journal_entry_fails_with_zero_amount() {
        let account_id = Uuid::new_v4();
        let posting = Posting {
            account_id,
            amount: 0,
            direction: Direction::Credit,
            remark: None,
        };
        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            postings: vec![posting],
        };

        let mut states = HashMap::new();
        states.insert(account_id, valid_ledger_state(account_id));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::ZeroAmountPosting);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_negative_amount() {
        let account_id = Uuid::new_v4();
        let posting = Posting {
            account_id,
            amount: -100,
            direction: Direction::Credit,
            remark: None,
        };
        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            postings: vec![posting],
        };

        let mut states = HashMap::new();
        states.insert(account_id, valid_ledger_state(account_id));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::ZeroAmountPosting);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_imbalanced_entry() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 50,
                direction: Direction::Debit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            postings,
        };

        let mut states = HashMap::new();
        states.insert(account_id_1, valid_ledger_state(account_id_1));
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::ImbalancedJournalEntry);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_account_not_found() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            postings,
        };

        let mut states = HashMap::new();
        states.insert(account_id_1, valid_ledger_state(account_id_1));
        // Missing account_id_2

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::AccountNotFound);
    }

    #[test]
    fn test_apply_journal_entry_fails_with_insufficient_funds() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 2000,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 2000,
                direction: Direction::Credit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            postings,
        };

        let mut states = HashMap::new();
        states.insert(account_id_1, valid_ledger_state(account_id_1));
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        let result = apply_journal_entry(&journal_entry, states);

        assert_eq!(result.unwrap_err(), LedgerError::InsufficientFunds);
    }

    #[test]
    fn test_apply_journal_entry_success() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Test Entry".to_string(),
            timestamp: Utc::now(),
            postings,
        };

        let mut states = HashMap::new();
        states.insert(account_id_1, valid_ledger_state(account_id_1));
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        let result = apply_journal_entry(&journal_entry, states.clone());

        assert!(result.is_ok());

        let new_states = result.unwrap();

        assert_eq!(new_states.get(&account_id_1).unwrap().current_balance, 900);
        assert_eq!(new_states.get(&account_id_1).unwrap().version, 2);

        assert_eq!(new_states.get(&account_id_2).unwrap().current_balance, 1100);
        assert_eq!(new_states.get(&account_id_2).unwrap().version, 2);
    }

    proptest! {
        #[test]
        fn property_test_balance_integrity(
            initial_balance in 100..10_000i64,
            transfer_amount in 1..100i64
        ) {
             let account_id_1 = Uuid::new_v4();
             let account_id_2 = Uuid::new_v4();

             let postings = vec![
                Posting {
                    account_id: account_id_1,
                    amount: transfer_amount,
                    direction: Direction::Debit,
                    remark: None,
                },
                Posting {
                    account_id: account_id_2,
                    amount: transfer_amount,
                    direction: Direction::Credit,
                    remark: None,
                },
            ];

            let journal_entry = JournalEntry {
                id: Uuid::new_v4(),
                description: "Proptest Entry".to_string(),
                timestamp: Utc::now(),
                postings,
            };

            let mut states = HashMap::new();

            let mut state_1 = valid_ledger_state(account_id_1);
            state_1.current_balance = initial_balance;
            states.insert(account_id_1, state_1);

            let mut state_2 = valid_ledger_state(account_id_2);
            state_2.current_balance = initial_balance;
            states.insert(account_id_2, state_2);

            let result = apply_journal_entry(&journal_entry, states.clone());
            prop_assert!(result.is_ok());

            let new_states = result.unwrap();
            let new_state_1 = new_states.get(&account_id_1).unwrap();
            let new_state_2 = new_states.get(&account_id_2).unwrap();

            // Property: Total funds should remain constant
            prop_assert_eq!(
                new_state_1.current_balance + new_state_2.current_balance,
                initial_balance + initial_balance
            );
        }
    }

    #[test]
    fn test_apply_journal_entry_respects_overdraft_and_holds() {
        let account_id_1 = Uuid::new_v4(); // Sender
        let account_id_2 = Uuid::new_v4(); // Receiver

        let mut state_1 = valid_ledger_state(account_id_1);
        // Current: 1000. Holds: 400. Limit: 500.
        // Available = 1000 - 400 + 500 = 1100.
        state_1.current_balance = 1000;
        state_1.pending_holds = 400;
        state_1.overdraft_limit = 500;

        let mut states = HashMap::new();
        states.insert(account_id_1, state_1);
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        // Attempt to debit 1200 (Exceeds available of 1100) -> Should Fail
        let fail_postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 1200,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 1200,
                direction: Direction::Credit,
                remark: None,
            },
        ];
        let fail_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Fail".to_string(),
            timestamp: Utc::now(),
            postings: fail_postings,
        };

        let fail_result = apply_journal_entry(&fail_entry, states.clone());
        assert_eq!(fail_result.unwrap_err(), LedgerError::InsufficientFunds);

        // Attempt to debit 1100 (Exactly drains available) -> Should Pass, leaving balance at -100
        let pass_postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 1100,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 1100,
                direction: Direction::Credit,
                remark: None,
            },
        ];
        let pass_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Pass".to_string(),
            timestamp: Utc::now(),
            postings: pass_postings,
        };

        let pass_result = apply_journal_entry(&pass_entry, states);
        assert!(pass_result.is_ok());
        assert_eq!(
            pass_result
                .unwrap()
                .get(&account_id_1)
                .unwrap()
                .current_balance,
            -100
        );
    }

    #[test]
    fn test_apply_journal_entry_evaluates_net_impact_atomically() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        let mut state_1 = valid_ledger_state(account_id_1);
        state_1.current_balance = 1000; // Available is 1000

        let mut states = HashMap::new();
        states.insert(account_id_1, state_1);
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        // Two debits of 600 against the same account. Total debit = 1200. Should fail.
        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 600,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_1,
                amount: 600,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 1200,
                direction: Direction::Credit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Net Impact Test".to_string(),
            timestamp: Utc::now(),
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
        };
        // 1000 - 400 + 500 = 1100
        assert_eq!(state.available_balance().unwrap(), 1100);

        let state_negative = LedgerState {
            id: Uuid::new_v4(),
            current_balance: -200, // Deep in overdraft
            pending_holds: 100,
            overdraft_limit: 500,
            version: 1,
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

        let mut states = HashMap::new();
        states.insert(account_id_1, state_1);
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Overflow Test".to_string(),
            timestamp: Utc::now(),
            postings,
        };

        let result = apply_journal_entry(&journal_entry, states);
        assert_eq!(result.unwrap_err(), LedgerError::ArithmeticOverflow);
    }

    #[test]
    fn test_apply_journal_entry_version_overflow() {
        let account_id_1 = Uuid::new_v4();
        let account_id_2 = Uuid::new_v4();

        let mut state_1 = valid_ledger_state(account_id_1);
        // Set version to maximum possible i32 to trigger overflow on increment
        state_1.version = i32::MAX;

        let mut states = HashMap::new();
        states.insert(account_id_1, state_1);
        states.insert(account_id_2, valid_ledger_state(account_id_2));

        let postings = vec![
            Posting {
                account_id: account_id_1,
                amount: 100,
                direction: Direction::Debit,
                remark: None,
            },
            Posting {
                account_id: account_id_2,
                amount: 100,
                direction: Direction::Credit,
                remark: None,
            },
        ];

        let journal_entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Version Overflow Test".to_string(),
            timestamp: Utc::now(),
            postings,
        };

        let result = apply_journal_entry(&journal_entry, states);
        assert_eq!(result.unwrap_err(), LedgerError::ArithmeticOverflow);
    }
}
