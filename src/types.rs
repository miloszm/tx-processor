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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputRecord {
    pub type_op: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Decimal,
}

pub const OUTPUT_HEADER: &str = "client,available,held,total,locked";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClientData {
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutputRecord {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransactionRecord {
    pub type_op: TxRecordTypeOp,
    pub client: u16,
    pub amount: Decimal,
    pub state: TxState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxRecordTypeOp {
    Deposit,
    Withdrawal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TxState {
    Active,
    Disputed,
    Resolved,
    ChargedBack,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OpOutcome {
    Applied,
    Rejected(RejectReason),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RejectReason {
    InsufficientFunds,
    InsufficientFundsForDispute,
    UnknownClient,
    DisputedTxNotFound,
    DisputedTxWrongClient,
    TxNotDisputed,
    TxNotActive,
    InsufficientHeldFunds,
    AccountLocked,
}
