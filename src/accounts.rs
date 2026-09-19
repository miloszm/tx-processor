use crate::error::TxProError;
use crate::types::{
    CLIENTS_HEADER, ClientData, OpOutcome, OutputRecord, RejectReason, TransactionRecord,
    TxRecordTypeOp, TxState,
};
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry::{Occupied, Vacant};

pub struct Accounts {
    data: BTreeMap<u16, ClientData>,
    txs: BTreeMap<u32, TransactionRecord>,
}

impl Accounts {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::<u16, ClientData>::new(),
            txs: BTreeMap::<u32, TransactionRecord>::new(),
        }
    }

    pub fn deposit(
        &mut self,
        client: u16,
        amount: Decimal,
        tx: u32,
    ) -> Result<OpOutcome, TxProError> {
        let amount = amount.trunc_with_scale(4);
        match self.data.entry(client) {
            Vacant(e) => {
                e.insert(ClientData {
                    available: amount,
                    held: Decimal::ZERO,
                    total: amount,
                    locked: false,
                });
            }
            Occupied(mut e) => {
                if e.get().locked {
                    return Ok(OpOutcome::Rejected(RejectReason::AccountLocked));
                }
                e.get_mut().available += amount;
                e.get_mut().total += amount;
            }
        }
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: TxRecordTypeOp::Deposit,
                client,
                amount,
                state: TxState::Active,
            },
        );
        Ok(OpOutcome::Applied)
    }

    pub fn withdrawal(
        &mut self,
        client: u16,
        amount: Decimal,
        tx: u32,
    ) -> Result<OpOutcome, TxProError> {
        match self.data.get_mut(&client) {
            None => {
                return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
            }
            Some(data) => {
                if data.locked {
                    return Ok(OpOutcome::Rejected(RejectReason::AccountLocked));
                }
                if data.available < amount {
                    return Ok(OpOutcome::Rejected(RejectReason::InsufficientFunds));
                }
                data.available -= amount;
                data.total -= amount
            }
        }
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: TxRecordTypeOp::Withdrawal,
                client,
                amount,
                state: TxState::Active,
            },
        );
        Ok(OpOutcome::Applied)
    }

    pub fn dispute(&mut self, client: u16, tx: u32) -> Result<OpOutcome, TxProError> {
        let Some(data) = self.data.get_mut(&client) else {
            return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
        };
        let Some(tr) = self.txs.get(&tx) else {
            return Ok(OpOutcome::Rejected(RejectReason::DisputedTxNotFound));
        };

        if tr.state != TxState::Active {
            return Ok(OpOutcome::Rejected(RejectReason::TxNotActive));
        }
        if tr.client != client {
            return Ok(OpOutcome::Rejected(RejectReason::DisputedTxWrongClient));
        }

        let amount = tr.amount;
        if data.available < amount {
            return Ok(OpOutcome::Rejected(
                RejectReason::InsufficientFundsForDispute,
            ));
        }

        data.available -= amount;
        data.held += amount;
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: tr.type_op,
                client,
                amount,
                state: TxState::Disputed,
            },
        );
        Ok(OpOutcome::Applied)
    }

    pub fn resolve(&mut self, client: u16, tx: u32) -> Result<OpOutcome, TxProError> {
        let Some(data) = self.data.get_mut(&client) else {
            return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
        };
        let Some(tr) = self.txs.get(&tx) else {
            return Ok(OpOutcome::Rejected(RejectReason::DisputedTxNotFound));
        };
        if tr.state != TxState::Disputed {
            return Ok(OpOutcome::Rejected(RejectReason::TxNotDisputed));
        }
        if tr.client != client {
            return Ok(OpOutcome::Rejected(RejectReason::DisputedTxWrongClient));
        }
        let amount = tr.amount;
        if data.held < amount {
            return Ok(OpOutcome::Rejected(RejectReason::InsufficientHeldFunds));
        }
        data.available += amount;
        data.held -= amount;
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: tr.type_op,
                client,
                amount,
                state: TxState::Resolved,
            },
        );
        Ok(OpOutcome::Applied)
    }

    pub fn chargeback(&mut self, client: u16, tx: u32) -> Result<OpOutcome, TxProError> {
        let Some(data) = self.data.get_mut(&client) else {
            return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
        };
        let Some(tr) = self.txs.get(&tx) else {
            return Ok(OpOutcome::Rejected(RejectReason::DisputedTxNotFound));
        };
        if tr.state != TxState::Disputed {
            return Ok(OpOutcome::Rejected(RejectReason::TxNotDisputed));
        }
        if tr.client != client {
            return Ok(OpOutcome::Rejected(RejectReason::DisputedTxWrongClient));
        }
        let amount = tr.amount;
        if data.held < amount {
            return Ok(OpOutcome::Rejected(RejectReason::InsufficientHeldFunds));
        }
        data.held -= amount;
        data.total -= amount;
        data.locked = true;
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: tr.type_op,
                client,
                amount,
                state: TxState::ChargedBack,
            },
        );
        Ok(OpOutcome::Applied)
    }

    pub fn client_records(&self) -> impl Iterator<Item = OutputRecord> + '_ {
        self.data.iter().map(|(&client, data)| OutputRecord {
            client,
            available: data.available,
            held: data.held,
            total: data.total,
            locked: data.locked,
        })
    }

    // todo
    pub fn print_accounts(&self) {
        for s in CLIENTS_HEADER {
            print!("{s} ");
        }
        println!();
        for (client, data) in self.data.range(..) {
            println!(
                "{},{},{},{},{}",
                client, data.available, data.held, data.total, data.locked
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    // ------------------------------------------------------------------
    // Helpers
    // ------------------------------------------------------------------

    fn accounts() -> Accounts {
        Accounts::new()
    }

    fn records_vec(acc: &Accounts) -> Vec<OutputRecord> {
        acc.client_records().collect()
    }

    fn assert_client(
        acc: &Accounts,
        client: u16,
        available: Decimal,
        held: Decimal,
        total: Decimal,
        locked: bool,
    ) {
        let records = records_vec(acc);
        let rec = records
            .iter()
            .find(|r| r.client == client)
            .unwrap_or_else(|| panic!("no record for client {client}"));

        assert_eq!(
            rec.available, available,
            "available mismatch for client {client}"
        );
        assert_eq!(rec.held, held, "held mismatch for client {client}");
        assert_eq!(rec.total, total, "total mismatch for client {client}");
        assert_eq!(rec.locked, locked, "locked mismatch for client {client}");
    }

    fn assert_no_client(acc: &Accounts, client: u16) {
        let records = records_vec(acc);
        assert!(
            records.iter().all(|r| r.client != client),
            "expected no record for client {client}, found one"
        );
    }

    fn assert_tx_state(acc: &Accounts, tx: u32, state: TxState) {
        let tr = acc.txs.get(&tx).expect("tx should exist");
        assert_eq!(tr.state, state, "state mismatch for tx {tx}");
    }

    fn assert_tx_op(acc: &Accounts, tx: u32, op: TxRecordTypeOp) {
        let tr = acc.txs.get(&tx).expect("tx should exist");
        assert_eq!(tr.type_op, op, "type_op mismatch for tx {tx}");
    }

    // ------------------------------------------------------------------
    // Deposit
    // ------------------------------------------------------------------

    #[test]
    fn deposit_creates_new_client() {
        let mut acc = accounts();
        let outcome = acc.deposit(1, dec!(10.00), 100).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(10.00), dec!(0), dec!(10.00), false);
        assert_eq!(records_vec(&acc).len(), 1);
        assert_tx_state(&acc, 100, TxState::Active);
        assert_tx_op(&acc, 100, TxRecordTypeOp::Deposit);
    }

    #[test]
    fn deposit_accumulates_for_existing_client() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.deposit(1, dec!(5.50), 101).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(15.50), dec!(0), dec!(15.50), false);
    }

    #[test]
    fn deposit_separate_clients_are_independent() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.deposit(2, dec!(20.00), 101).unwrap();

        assert_client(&acc, 1, dec!(10.00), dec!(0), dec!(10.00), false);
        assert_client(&acc, 2, dec!(20.00), dec!(0), dec!(20.00), false);
    }

    #[test]
    fn deposit_rejects_locked_account() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();
        // Client 1 is now locked.

        let outcome = acc.deposit(1, dec!(5.00), 101).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::AccountLocked));
        // State unchanged; failed deposit not recorded.
        assert_client(&acc, 1, dec!(0), dec!(0), dec!(0), true);
        assert!(acc.txs.get(&101).is_none());
    }

    // ------------------------------------------------------------------
    // Withdrawal
    // ------------------------------------------------------------------

    #[test]
    fn withdrawal_rejects_unknown_client() {
        let mut acc = accounts();
        let outcome = acc.withdrawal(1, dec!(5.00), 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::UnknownClient));
        assert_no_client(&acc, 1);
        assert!(acc.txs.get(&100).is_none());
    }

    #[test]
    fn withdrawal_rejects_insufficient_funds() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.withdrawal(1, dec!(15.00), 101).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::InsufficientFunds)
        );
        assert_client(&acc, 1, dec!(10.00), dec!(0), dec!(10.00), false);
        assert!(acc.txs.get(&101).is_none());
    }

    #[test]
    fn withdrawal_rejects_locked_account() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();
        // Client 1 is now locked, available=0, held=0, total=0.

        let outcome = acc.withdrawal(1, dec!(0.00), 101).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::AccountLocked));
        assert!(acc.txs.get(&101).is_none());
    }

    #[test]
    fn withdrawal_succeeds_with_exact_balance() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.withdrawal(1, dec!(10.00), 101).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(0), dec!(0), dec!(0), false);
    }

    #[test]
    fn withdrawal_decreases_available_and_total() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.withdrawal(1, dec!(3.50), 101).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(6.50), dec!(0), dec!(6.50), false);
    }

    // ------------------------------------------------------------------
    // Dispute
    // ------------------------------------------------------------------

    #[test]
    fn dispute_rejects_unknown_client() {
        let mut acc = accounts();
        let outcome = acc.dispute(1, 100).unwrap();
        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::UnknownClient));
    }

    #[test]
    fn dispute_rejects_unknown_tx() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.dispute(1, 999).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::DisputedTxNotFound)
        );
    }

    #[test]
    fn dispute_moves_funds_to_held() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.dispute(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(0), dec!(10.00), dec!(10.00), false);
        assert_tx_state(&acc, 100, TxState::Disputed);
        // type_op is preserved as the original op.
        assert_tx_op(&acc, 100, TxRecordTypeOp::Deposit);
    }

    #[test]
    fn dispute_rejects_when_available_less_than_amount() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.withdrawal(1, dec!(8.00), 101).unwrap();
        // available = 2.00

        let outcome = acc.dispute(1, 100).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::InsufficientFundsForDispute)
        );
        assert_client(&acc, 1, dec!(2.00), dec!(0), dec!(2.00), false);
    }

    #[test]
    fn dispute_cannot_be_applied_twice() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();

        let outcome = acc.dispute(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotActive));
        assert_client(&acc, 1, dec!(0), dec!(10.00), dec!(10.00), false);
    }

    #[test]
    fn dispute_rejects_wrong_client() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.deposit(2, dec!(5.00), 200).unwrap();

        let outcome = acc.dispute(2, 100).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::DisputedTxWrongClient)
        );
        assert_client(&acc, 2, dec!(5.00), dec!(0), dec!(5.00), false);
        // Original tx left Active.
        assert_tx_state(&acc, 100, TxState::Active);
    }

    #[test]
    fn dispute_after_withdrawal_moves_held_correctly() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.withdrawal(1, dec!(3.00), 101).unwrap();
        // available = 7.00, total = 7.00

        let outcome = acc.dispute(1, 101).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        // Disputing a withdrawal moves 3.00 from available to held.
        assert_client(&acc, 1, dec!(4.00), dec!(3.00), dec!(7.00), false);
    }

    // ------------------------------------------------------------------
    // Resolve
    // ------------------------------------------------------------------

    #[test]
    fn resolve_rejects_unknown_client() {
        let mut acc = accounts();
        let outcome = acc.resolve(1, 100).unwrap();
        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::UnknownClient));
    }

    #[test]
    fn resolve_rejects_unknown_tx() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.resolve(1, 999).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::DisputedTxNotFound)
        );
    }

    #[test]
    fn resolve_rejects_tx_not_currently_disputed() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();

        let outcome = acc.resolve(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotDisputed));
        assert_tx_state(&acc, 100, TxState::Active);
    }

    #[test]
    fn resolve_rejects_wrong_client() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.deposit(2, dec!(5.00), 200).unwrap();

        let outcome = acc.resolve(2, 100).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::DisputedTxWrongClient)
        );
    }

    #[test]
    fn resolve_moves_funds_back_from_held() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();

        let outcome = acc.resolve(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(10.00), dec!(0), dec!(10.00), false);
        assert_tx_state(&acc, 100, TxState::Resolved);
        assert_tx_op(&acc, 100, TxRecordTypeOp::Deposit);
    }

    #[test]
    fn resolve_cannot_be_applied_twice() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.resolve(1, 100).unwrap();

        let outcome = acc.resolve(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotDisputed));
        assert_client(&acc, 1, dec!(10.00), dec!(0), dec!(10.00), false);
    }

    // ------------------------------------------------------------------
    // Chargeback
    // ------------------------------------------------------------------

    #[test]
    fn chargeback_rejects_unknown_client() {
        let mut acc = accounts();
        let outcome = acc.chargeback(1, 100).unwrap();
        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::UnknownClient));
    }

    #[test]
    fn chargeback_rejects_unknown_tx() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let outcome = acc.chargeback(1, 999).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::DisputedTxNotFound)
        );
    }

    #[test]
    fn chargeback_rejects_tx_not_currently_disputed() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();

        let outcome = acc.chargeback(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotDisputed));
        assert_tx_state(&acc, 100, TxState::Active);
    }

    #[test]
    fn chargeback_rejects_wrong_client() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.deposit(2, dec!(5.00), 200).unwrap();

        let outcome = acc.chargeback(2, 100).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(RejectReason::DisputedTxWrongClient)
        );
    }

    #[test]
    fn chargeback_removes_held_and_locks_account() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();

        let outcome = acc.chargeback(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_client(&acc, 1, dec!(0), dec!(0), dec!(0), true);
        assert_tx_state(&acc, 100, TxState::ChargedBack);
        assert_tx_op(&acc, 100, TxRecordTypeOp::Deposit);
    }

    #[test]
    fn chargeback_cannot_be_applied_twice() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();

        let outcome = acc.chargeback(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotDisputed));
        assert_client(&acc, 1, dec!(0), dec!(0), dec!(0), true);
    }

    #[test]
    fn resolve_after_chargeback_is_rejected() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();

        let outcome = acc.resolve(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotDisputed));
    }

    #[test]
    fn chargeback_after_resolve_is_rejected() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.resolve(1, 100).unwrap();

        let outcome = acc.chargeback(1, 100).unwrap();

        assert_eq!(outcome, OpOutcome::Rejected(RejectReason::TxNotDisputed));
    }

    // ------------------------------------------------------------------
    // client_records()
    // ------------------------------------------------------------------

    #[test]
    fn client_records_empty_state_is_empty() {
        let acc = accounts();
        assert_eq!(acc.client_records().count(), 0);
    }

    #[test]
    fn client_records_are_sorted_by_client_id() {
        let mut acc = accounts();
        acc.deposit(3, dec!(1.00), 100).unwrap();
        acc.deposit(1, dec!(1.00), 101).unwrap();
        acc.deposit(2, dec!(1.00), 102).unwrap();

        let ids: Vec<u16> = acc.client_records().map(|r| r.client).collect();
        assert_eq!(ids, vec![1, 2, 3]);
    }

    #[test]
    fn client_records_carry_locked_and_held() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();

        let records: Vec<_> = acc.client_records().collect();
        assert_eq!(records.len(), 1);
        assert!(records[0].locked);
        assert_eq!(records[0].held, dec!(0));
        assert_eq!(records[0].total, dec!(0));
    }

    // ------------------------------------------------------------------
    // Invariant: available + held == total
    // ------------------------------------------------------------------

    #[test]
    fn invariant_available_plus_held_equals_total() {
        let mut acc = accounts();

        acc.deposit(1, dec!(10.00), 1).unwrap();
        acc.deposit(1, dec!(5.00), 2).unwrap();
        acc.deposit(2, dec!(3.00), 3).unwrap();
        acc.dispute(1, 1).unwrap();
        acc.withdrawal(2, dec!(1.00), 4).unwrap();
        acc.resolve(1, 1).unwrap();
        acc.dispute(1, 2).unwrap();
        acc.chargeback(1, 2).unwrap();

        for rec in acc.client_records() {
            assert_eq!(
                rec.available + rec.held,
                rec.total,
                "invariant broken for client {}",
                rec.client
            );
        }
    }

    // ------------------------------------------------------------------
    // End-to-end scenarios
    // ------------------------------------------------------------------

    #[test]
    fn scenario_dispute_then_resolve_restores_original_balance() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();

        acc.dispute(1, 100).unwrap();
        assert_client(&acc, 1, dec!(0), dec!(10.00), dec!(10.00), false);

        acc.resolve(1, 100).unwrap();
        assert_client(&acc, 1, dec!(10.00), dec!(0), dec!(10.00), false);
    }

    #[test]
    fn scenario_dispute_then_chargeback_zeroes_and_locks() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();

        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();

        assert_client(&acc, 1, dec!(0), dec!(0), dec!(0), true);
    }

    #[test]
    fn scenario_multiple_deposits_partial_withdrawals() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 1).unwrap();
        acc.deposit(1, dec!(5.00), 2).unwrap();
        acc.withdrawal(1, dec!(3.00), 3).unwrap();

        assert_client(&acc, 1, dec!(12.00), dec!(0), dec!(12.00), false);
    }

    #[test]
    fn scenario_locked_account_rejects_all_external_ops() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        acc.chargeback(1, 100).unwrap();
        // Client 1 locked.

        assert_eq!(
            acc.deposit(1, dec!(1.00), 200).unwrap(),
            OpOutcome::Rejected(RejectReason::AccountLocked)
        );
        assert_eq!(
            acc.withdrawal(1, dec!(1.00), 201).unwrap(),
            OpOutcome::Rejected(RejectReason::AccountLocked)
        );
    }

    #[test]
    fn deposit_truncates_amount_to_four_decimal_places() {
        let mut acc = accounts();
        acc.deposit(1, dec!(1.23456789), 100).unwrap();
        assert_client(&acc, 1, dec!(1.2345), dec!(0), dec!(1.2345), false);
    }
}
