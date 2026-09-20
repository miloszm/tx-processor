mod tests {
    use crate::dispatcher::*;
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

    #[test]
    fn routes_deposit() {
        let mut acc = accounts();
        let r = record("deposit", 1, 100, dec!(10.00));

        let outcome = dispatch(&r, TypeOp::Deposit, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
        assert_eq!(acc.txs.get(&100).unwrap().type_op, TxRecordTypeOp::Deposit);
    }

    #[test]
    fn routes_withdrawal() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        let r = record("withdrawal", 1, 101, dec!(3.00));

        let outcome = dispatch(&r, TypeOp::Withdrawal, &mut acc).unwrap();

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

        let outcome = dispatch(&r, TypeOp::Dispute, &mut acc).unwrap();

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

        let outcome = dispatch(&r, TypeOp::Resolve, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
    }

    #[test]
    fn routes_chargeback() {
        let mut acc = accounts();
        acc.deposit(1, dec!(10.00), 100).unwrap();
        acc.dispute(1, 100).unwrap();
        let r = record("chargeback", 1, 100, dec!(0));

        let outcome = dispatch(&r, TypeOp::Chargeback, &mut acc).unwrap();

        assert_eq!(outcome, OpOutcome::Applied);
    }

    #[test]
    fn business_rejection_is_ok_not_err() {
        let mut acc = accounts();
        let r = record("withdrawal", 1, 100, dec!(1.00));

        let outcome = dispatch(&r, TypeOp::Withdrawal, &mut acc).unwrap();

        assert_eq!(
            outcome,
            OpOutcome::Rejected(crate::types::RejectReason::UnknownClient)
        );
    }
}
