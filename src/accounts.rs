use crate::error::TxProError;
use crate::types::{
    CLIENTS_HEADER, ClientData, OpOutcome, RejectReason, TransactionRecord, TypeOp,
};
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

    pub fn deposit(
        &mut self,
        client: u16,
        amount: Decimal,
        tx: u32,
    ) -> Result<OpOutcome, TxProError> {
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
                if data.available < amount {
                    return Ok(OpOutcome::Rejected(RejectReason::InsufficientFunds));
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
        Ok(OpOutcome::Applied)
    }

    pub fn dispute(&mut self, client: u16, tx: u32) -> Result<OpOutcome, TxProError> {
        match self.data.get_mut(&client) {
            None => {
                return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
            }
            Some(data) => {
                if let Some(tr) = self.txs.get(&tx) {
                    if let Some(amount) = tr.amount {
                        if data.available < amount {
                            return Ok(OpOutcome::Rejected(RejectReason::InsufficientFunds));
                        }
                        (*data).available -= amount;
                        (*data).held += amount;
                        self.txs.insert(
                            tx,
                            TransactionRecord {
                                type_op: TypeOp::Dispute,
                                client,
                                amount: Some(amount),
                            },
                        );
                    } else {
                        return Ok(OpOutcome::Rejected(RejectReason::DisputedTxLacksAmount));
                    }
                } else {
                    return Ok(OpOutcome::Rejected(RejectReason::DisputedTxNotFound));
                }
            }
        }

        Ok(OpOutcome::Applied)
    }

    pub fn resolve(&mut self, client: u16, tx: u32) -> Result<OpOutcome, TxProError> {
        match self.data.get_mut(&client) {
            None => {
                return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
            }
            Some(data) => {
                if let Some(tr) = self.txs.get(&tx) {
                    if tr.type_op != TypeOp::Dispute {
                        return Ok(OpOutcome::Rejected(RejectReason::TxNotDisputed));
                    }
                    if tr.client != client {
                        return Ok(OpOutcome::Rejected(RejectReason::DisputedTxWrongClient));
                    }
                    if let Some(amount) = tr.amount {
                        if data.held < amount {
                            return Ok(OpOutcome::Rejected(RejectReason::InsufficientHeldFunds));
                        }
                        (*data).available += amount;
                        (*data).held -= amount;
                        self.txs.insert(
                            tx,
                            TransactionRecord {
                                type_op: TypeOp::Resolve,
                                client,
                                amount: None,
                            },
                        );
                    } else {
                        return Ok(OpOutcome::Rejected(RejectReason::DisputedTxLacksAmount));
                    }
                } else {
                    return Ok(OpOutcome::Rejected(RejectReason::DisputedTxNotFound));
                }
            }
        }

        Ok(OpOutcome::Applied)
    }

    pub fn chargeback(&mut self, client: u16, tx: u32) -> Result<OpOutcome, TxProError> {
        match self.data.get_mut(&client) {
            None => {
                return Ok(OpOutcome::Rejected(RejectReason::UnknownClient));
            }
            Some(data) => {
                if let Some(tr) = self.txs.get(&tx) {
                    if tr.client != client {
                        return Ok(OpOutcome::Rejected(RejectReason::DisputedTxWrongClient));
                    }
                    if let Some(amount) = tr.amount {
                        if data.held < amount {
                            return Ok(OpOutcome::Rejected(RejectReason::InsufficientHeldFunds));
                        }
                        (*data).held -= amount;
                        (*data).total -= amount;
                        (*data).locked = true;
                        self.txs.insert(
                            tx,
                            TransactionRecord {
                                type_op: TypeOp::Resolve,
                                client,
                                amount: None,
                            },
                        );
                    } else {
                        return Ok(OpOutcome::Rejected(RejectReason::InsufficientHeldFunds));
                    }
                } else {
                    return Ok(OpOutcome::Rejected(RejectReason::DisputedTxNotFound));
                }
            }
        }

        Ok(OpOutcome::Applied)
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
