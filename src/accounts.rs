use std::collections::BTreeMap;
use std::collections::btree_map::Entry::{Occupied, Vacant};
use std::io::Write;

use rust_decimal::Decimal;

use crate::error::TxProError;
use crate::types::{
    ClientData, OUTPUT_HEADER, OpOutcome, OutputRecord, RejectReason, TransactionRecord,
    TxRecordTypeOp, TxState,
};

pub struct Accounts {
    pub(crate) data: BTreeMap<u16, ClientData>,
    pub(crate) txs: BTreeMap<u32, TransactionRecord>,
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
        let amount = amount.trunc_with_scale(4);
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

    /// Writes one CSV row per client to `out`.
    pub fn print_accounts<W: Write>(&self, out: &mut W) -> Result<(), TxProError> {
        writeln!(out, "{}", OUTPUT_HEADER)?;

        for record in self.client_records() {
            writeln!(
                out,
                "{},{},{},{},{}",
                record.client, record.available, record.held, record.total, record.locked,
            )?;
        }

        out.flush()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests;
