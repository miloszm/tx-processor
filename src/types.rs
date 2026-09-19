use rust_decimal::Decimal;

#[derive(Debug)]
pub enum TypeOp {
    Deposit,
    Withdrawal,
    Dispute,
    Resolve,
    Chargeback,
}

#[derive(Debug)]
pub struct InputRecord {
    pub type_op: String,
    pub client: u16,
    pub tx: u32,
    pub amount: Decimal,
}

#[derive(Debug)]
pub struct OutputRecord {
    pub client: u16,
    pub available: Decimal,
    pub held: Decimal,
    pub total: Decimal,
    pub locked: bool,
}

#[derive(Debug)]
pub struct Transaction {
    pub type_op: TypeOp,
    pub client: u16,
    pub tx: u32,
    pub amount: Option<Decimal>,
}
