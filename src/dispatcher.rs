use crate::accounts::Accounts;
use crate::error::TxProError;
use crate::types::{InputRecord, OpOutcome, TypeOp, UnknownTypeOp};

pub fn dispatch(r: &InputRecord, accounts: &mut Accounts) -> Result<OpOutcome, TxProError> {
    let type_op: TypeOp =
        r.type_op
            .parse()
            .map_err(|source: UnknownTypeOp| TxProError::BadTypeOp {
                value: r.type_op.clone(),
                source,
            })?;

    match type_op {
        TypeOp::Deposit => accounts.deposit(r.client, r.amount, r.tx),
        TypeOp::Withdrawal => accounts.withdrawal(r.client, r.amount, r.tx),
        TypeOp::Dispute => accounts.dispute(r.client, r.tx),
        TypeOp::Resolve => accounts.resolve(r.client, r.tx),
        TypeOp::Chargeback => accounts.chargeback(r.client, r.tx),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::TxRecordTypeOp;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;

    fn record(type_op: &str, client: u16, tx: u32, amount: Decimal) -> InputRecord {
        InputRecord {
            type_op: type_op.to_string(),
            client,
            tx,
            amount,
        }
    }

    fn accounts() -> Accounts {
        Accounts::new()
    }

    // --- routing: each op reaches the right Accounts method -----------------

    #[test]
    fn routes_deposit() {
        let mut acc = accounts();
        let r = record("deposit", 1, 100, dec!(10.00));

        let outcome = dispatch(&r, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_eq!(acc.txs.get(&100).unwrap().type_op, TxRecordTypeOp::Deposit);
    }

    #[test]
    fn routes_withdrawal() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let r = record("withdrawal", 1, 101, dec!(3.00));

        let outcome = dispatch(&r, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_eq!(
            acc.txs.get(&101).unwrap().type_op,
            TxRecordTypeOp::Withdrawal
        );
    }

    #[test]
    fn routes_dispute() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let r = record("dispute", 1, 100, dec!(0));

        let outcome = dispatch(&r, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        // After a dispute, available is 0 and held is 10.
        // (assert via client_records if you prefer)
    }

    #[test]
    fn routes_resolve() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        let r = record("resolve", 1, 100, dec!(0));

        let outcome = dispatch(&r, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
    }

    #[test]
    fn routes_chargeback() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        let r = record("chargeback", 1, 100, dec!(0));

        let outcome = dispatch(&r, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
    }

    #[test]
    fn rejects_unknown_type_op() {
        let mut acc = accounts();
        let r = record("bogus", 1, 100, dec!(1.00));

        let err = dispatch(&r, &mut acc).unwrap_err();

        match err {
            TxProError::BadTypeOp { value, source } => {
                assert_eq!(value, "bogus");
                assert_eq!(source.0, "bogus");
            }
            other => panic!("expected BadTypeOp, got {other:?}"),
        }
        assert!(acc.txs.is_empty());
    }

    #[test]
    fn business_rejection_is_ok_not_err() {
        let mut acc = accounts();
        let r = record("withdrawal", 1, 100, dec!(1.00));

        let outcome = dispatch(&r, &mut acc).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(crate::types::RejectReason::UnknownClient)
        );
    }
}
