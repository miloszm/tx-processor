use crate::error::TxProError;
use crate::types::{ClientData, TransactionRecord, TypeOp};
use TxProError::BadOp;
use rust_decimal::Decimal;
use std::collections::BTreeMap;
use std::collections::btree_map::Entry::{Occupied, Vacant};

pub struct Accounts {
    pub data: BTreeMap<u16, ClientData>,
    pub txs: BTreeMap<u32, TransactionRecord>,
}

impl Accounts {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::<u16, ClientData>::new(),
            txs: BTreeMap::<u32, TransactionRecord>::new(),
        }
    }

    pub fn deposit(&mut self, client: u16, amount: Decimal, tx: u32) -> Result<(), TxProError> {
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
                e.get_mut().available += amount;
                e.get_mut().total += amount;
            }
        }
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: TypeOp::Deposit,
                client,
                amount: Some(amount),
            },
        );
        Ok(())
    }

    pub fn withdrawal(&mut self, client: u16, amount: Decimal, tx: u32) -> Result<(), TxProError> {
        match self.data.get_mut(&client) {
            None => {
                return Err(BadOp {
                    value: "withdrawal rejected, client not found".into(),
                });
            }
            Some(data) => {
                if amount < data.available {
                    return Err(BadOp {
                        value: "withdrawal aborted, insufficent funds".into(),
                    });
                }
                (*data).available -= amount;
                (*data).total -= amount
            }
        }
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: TypeOp::Withdrawal,
                client,
                amount: Some(amount),
            },
        );
        Ok(())
    }

    pub fn dispute(&mut self, client: u16, tx: u32) -> Result<(), TxProError> {
        match self.data.get_mut(&client) {
            None => {
                return Err(BadOp {
                    value: "dispute rejected, client not found".into(),
                });
            }
            Some(data) => {
                if let Some(tx) = self.txs.get(&tx) {
                    if let Some(amount) = tx.amount {
                        if data.available < amount {
                            return Err(BadOp {
                                value: "dispute rejected, insufficient funds".into(),
                            });
                        }
                        (*data).available -= amount;
                        (*data).held += amount;
                    } else {
                        return Err(BadOp {
                            value: "dispute rejected, transaction does not have amount".into(),
                        });
                    }
                } else {
                    return Err(BadOp {
                        value: "dispute rejected, transaction not found".into(),
                    });
                }
            }
        }
        self.txs.insert(
            tx,
            TransactionRecord {
                type_op: TypeOp::Dispute,
                client,
                amount: None,
            },
        );

        Ok(())
    }
}
