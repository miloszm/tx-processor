use std::collections::btree_map::Entry::{Occupied, Vacant};
use std::collections::BTreeMap;
use rust_decimal::Decimal;
use crate::error::TxProError;
use crate::types::ClientData;

pub struct Accounts {
    pub data: BTreeMap<u16, ClientData>
}

impl Accounts {
    pub fn new() -> Self {
        Self {
            data: BTreeMap::<u16, ClientData>::new(),
        }
    }

    pub fn deposit(&mut self, client: u16, amount: Decimal, tx: u32) -> Result<(), TxProError> {
        match self.data.entry(client){
            Vacant(e) => {
                e.insert(ClientData {
                    available: amount,
                    held: Decimal::ZERO,
                    total: amount,
                    locked: false
                });
            }
            Occupied(mut e) => {
                e.get_mut().available += amount;
                e.get_mut().total += amount;
            }
        }
        Ok(())
    }
}
