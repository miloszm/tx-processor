use rust_decimal::Decimal;
use std::str::FromStr;
use thiserror::Error;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeOp {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug, Error)]
#[error("unknown operation type `{0}`")]
pub struct UnknownTypeOp(pub String);

impl FromStr for TypeOp {
    type Err = UnknownTypeOp;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "deposit" => Ok(TypeOp::Deposit),
            "withdrawal" => Ok(TypeOp::Withdrawal),
            "dispute" => Ok(TypeOp::Dispute),
            "resolve" => Ok(TypeOp::Resolve),
            "chargeback" => Ok(TypeOp::Chargeback),
            other => Err(UnknownTypeOp(other.to_string())),
        }
    }
}

#[derive(Debug)]
pub struct InputRecord {
    pub type_op: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Decimal,
}

pub const CLIENTS_HEADER: &[&str] = &["client", "available", "held", "total", "locked"];

#[derive(Debug, Clone)]
pub struct ClientData {
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[derive(Debug)]
pub struct TransactionRecord {
    pub type_op: TypeOp,
    pub client: u16,
    pub amount: Option<Decimal>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpOutcome {
    Applied,
    Rejected(RejectReason),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RejectReason {
    InsufficientFunds,
    UnknownClient,
    DisputedTxLacksAmount,
    DisputedTxNotFound,
    DisputedTxWrongClient,
    TxNotDisputed,
    InsufficientHeldFunds,
}
