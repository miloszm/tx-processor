use crate::accounts::Accounts;
use crate::error::TxProError;
use crate::types::{InputRecord, TypeOp, UnknownTypeOp};

pub struct InputRecordProcessor;

impl InputRecordProcessor {
    pub fn process(r: InputRecord, accounts: &mut Accounts) -> Result<(), TxProError> {
        let type_op: TypeOp =
            r.type_op
                .parse()
                .map_err(|source: UnknownTypeOp| TxProError::BadTypeOp {
                    value: r.type_op,
                    source,
                })?;

        match type_op {
            TypeOp::Deposit => {
                accounts.deposit(r.client, r.amount, r.tx)?;
            }
            TypeOp::Withdrawal => {
                accounts.withdrawal(r.client, r.amount, r.tx)?;
            }
            TypeOp::Dispute => {
                accounts.dispute(r.client, r.tx)?;
            }
            TypeOp::Resolve => {
                accounts.resolve(r.client, r.tx)?;
            }
            TypeOp::Chargeback => {
                accounts.chargeback(r.client, r.tx)?;
            }
        }

        Ok(())
    }
}
