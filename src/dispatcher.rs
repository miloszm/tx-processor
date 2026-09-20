use crate::accounts::Accounts;
use crate::error::TxProError;
use crate::types::{InputRecord, OpOutcome, TypeOp};

pub fn dispatch(
    r: &InputRecord,
    op: TypeOp,
    accounts: &mut Accounts,
) -> Result<OpOutcome, TxProError> {
    match op {
        TypeOp::Deposit => accounts.deposit(r.client, r.amount, r.tx),
        TypeOp::Withdrawal => accounts.withdrawal(r.client, r.amount, r.tx),
        TypeOp::Dispute => accounts.dispute(r.client, r.tx),
        TypeOp::Resolve => accounts.resolve(r.client, r.tx),
        TypeOp::Chargeback => accounts.chargeback(r.client, r.tx),
    }
}

#[cfg(test)]
mod tests;
