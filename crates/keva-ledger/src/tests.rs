#[cfg(test)]
mod tests {
    use crate::domain::{
        Direction, JournalEntry, LedgerError, LedgerState, Posting, apply_journal_entry,
    };
    use chrono::Utc;
    use proptest::prelude::*;
    use uuid::Uuid;

    // --- Helper function for tests ---
    fn create_account(id: Uuid, balance: i64, limit: i64) -> LedgerState {
        LedgerState {
            id,
            current_balance: balance,
            overdraft_limit: limit,
            version: 1,
        }
    }

    // --- Basic Constraints & Behaviors ---

    #[test]
    fn test_balanced_journal_entry_succeeds() {
        let acc1_id = Uuid::new_v4();
        let acc2_id = Uuid::new_v4();

        let entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Standard Transfer".to_string(),
            timestamp: Utc::now(),
            postings: vec![
                Posting {
                    account_id: acc1_id,
                    amount: 100,
                    direction: Direction::Debit,
                    remark: None,
                },
                Posting {
                    account_id: acc2_id,
                    amount: 100,
                    direction: Direction::Credit,
                    remark: None,
                },
            ],
        };

        let accounts = vec![
            create_account(acc1_id, 1000, 0),
            create_account(acc2_id, 1000, 0),
        ];

        let result = apply_journal_entry(&entry, accounts).expect("Should succeed");

        let a1 = result.iter().find(|a| a.id == acc1_id).unwrap();
        let a2 = result.iter().find(|a| a.id == acc2_id).unwrap();

        assert_eq!(a1.current_balance, 900); // 1000 - 100
        assert_eq!(a2.current_balance, 1100); // 1000 + 100
        assert_eq!(a1.version, 2);
        assert_eq!(a2.version, 2);
    }

    #[test]
    fn test_imbalanced_journal_entry_fails() {
        let acc1_id = Uuid::new_v4();
        let acc2_id = Uuid::new_v4();

        let entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Imbalanced Transfer".to_string(),
            timestamp: Utc::now(),
            postings: vec![
                Posting {
                    account_id: acc1_id,
                    amount: 100, // Debit 100
                    direction: Direction::Debit,
                    remark: None,
                },
                Posting {
                    account_id: acc2_id,
                    amount: 90, // Credit 90 (Mismatch!)
                    direction: Direction::Credit,
                    remark: None,
                },
            ],
        };

        let accounts = vec![
            create_account(acc1_id, 1000, 0),
            create_account(acc2_id, 1000, 0),
        ];

        let result = apply_journal_entry(&entry, accounts);
        assert_eq!(result.unwrap_err(), LedgerError::ImbalancedJournalEntry);
    }

    #[test]
    fn test_missing_account_fails() {
        let existing_acc_id = Uuid::new_v4();
        let missing_acc_id = Uuid::new_v4();

        let entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Missing Account TX".to_string(),
            timestamp: Utc::now(),
            postings: vec![
                Posting {
                    account_id: existing_acc_id,
                    amount: 100,
                    direction: Direction::Debit,
                    remark: None,
                },
                Posting {
                    account_id: missing_acc_id,
                    amount: 100,
                    direction: Direction::Credit,
                    remark: None,
                },
            ],
        };

        let accounts = vec![create_account(existing_acc_id, 1000, 0)];

        let result = apply_journal_entry(&entry, accounts);
        assert_eq!(
            result.unwrap_err(),
            LedgerError::LedgerNotFound(missing_acc_id)
        );
    }

    #[test]
    fn test_multi_leg_transaction_succeeds() {
        // Testing a scenario like a payment that incurs a fee routed to a fee account
        let sender_id = Uuid::new_v4();
        let receiver_id = Uuid::new_v4();
        let fee_pool_id = Uuid::new_v4();

        let entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Payment with Fee".to_string(),
            timestamp: Utc::now(),
            postings: vec![
                Posting {
                    account_id: sender_id,
                    amount: 105, // Customer spends 100 + 5 fee
                    direction: Direction::Debit,
                    remark: None,
                },
                Posting {
                    account_id: receiver_id,
                    amount: 100, // Merchant receives 100
                    direction: Direction::Credit,
                    remark: None,
                },
                Posting {
                    account_id: fee_pool_id,
                    amount: 5, // Bank collects 5 fee
                    direction: Direction::Credit,
                    remark: None,
                },
            ],
        };

        let accounts = vec![
            create_account(sender_id, 1000, 0),
            create_account(receiver_id, 0, 0),
            create_account(fee_pool_id, 0, 0),
        ];

        let result = apply_journal_entry(&entry, accounts).expect("Multi-leg should succeed");

        let sender = result.iter().find(|a| a.id == sender_id).unwrap();
        let receiver = result.iter().find(|a| a.id == receiver_id).unwrap();
        let fee_pool = result.iter().find(|a| a.id == fee_pool_id).unwrap();

        assert_eq!(sender.current_balance, 895);
        assert_eq!(receiver.current_balance, 100);
        assert_eq!(fee_pool.current_balance, 5);

        // Assert OCC versions all incremented
        assert_eq!(sender.version, 2);
        assert_eq!(receiver.version, 2);
        assert_eq!(fee_pool.version, 2);
    }

    #[test]
    fn test_transaction_bringing_balance_negative_succeeds() {
        // Because checking boundaries are not done in pure ledger parsing unless overdraft checks
        // are explicitly called inside process_journal_entry. Since the code strictly handles OCC
        // updating, this checks mathematical allowance of negative ints
        let acc1_id = Uuid::new_v4();
        let acc2_id = Uuid::new_v4();

        let entry = JournalEntry {
            id: Uuid::new_v4(),
            description: "Standard Transfer".to_string(),
            timestamp: Utc::now(),
            postings: vec![
                Posting {
                    account_id: acc1_id,
                    amount: 200, // Trying to withdraw 200 from an account with 100
                    direction: Direction::Debit,
                    remark: None,
                },
                Posting {
                    account_id: acc2_id,
                    amount: 200,
                    direction: Direction::Credit,
                    remark: None,
                },
            ],
        };

        let accounts = vec![
            create_account(acc1_id, 100, 0), // Will go to -100
            create_account(acc2_id, 1000, 0),
        ];

        let result =
            apply_journal_entry(&entry, accounts).expect("Mathematical operation should succeed");
        let a1 = result.iter().find(|a| a.id == acc1_id).unwrap();
        assert_eq!(a1.current_balance, -100);
    }

    // --- Property Based Tests ---

    proptest! {
        #[test]
        fn test_proptest_balanced_entry_always_succeeds(
            amount in 0i64..1_000_000_000_000i64,
            initial_b1 in -1_000_000_000i64..1_000_000_000i64,
            initial_b2 in -1_000_000_000i64..1_000_000_000i64,
            v1 in 1i32..10_000i32,
            v2 in 1i32..10_000i32,
        ) {
            let id1 = Uuid::new_v4();
            let id2 = Uuid::new_v4();

            let entry = JournalEntry {
                id: Uuid::new_v4(),
                description: "Proptest Transfer".to_string(),
                timestamp: Utc::now(),
                postings: vec![
                    Posting {
                        account_id: id1,
                        amount,
                        direction: Direction::Debit,
                        remark: None,
                    },
                    Posting {
                        account_id: id2,
                        amount,
                        direction: Direction::Credit,
                        remark: None,
                    },
                ],
            };

            let accounts = vec![
                LedgerState { id: id1, current_balance: initial_b1, overdraft_limit: 0, version: v1 },
                LedgerState { id: id2, current_balance: initial_b2, overdraft_limit: 0, version: v2 },
            ];

            let result = apply_journal_entry(&entry, accounts).expect("should succeed");

            let a1 = result.iter().find(|a| a.id == id1).unwrap();
            let a2 = result.iter().find(|a| a.id == id2).unwrap();

            assert_eq!(a1.current_balance, initial_b1 - amount);
            assert_eq!(a2.current_balance, initial_b2 + amount);
            assert_eq!(a1.version, v1 + 1);
            assert_eq!(a2.version, v2 + 1);
        }
    }
}
