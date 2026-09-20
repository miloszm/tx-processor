use csv::StringRecord;
use rust_decimal::Decimal;

use crate::error;
use crate::types::{InputRecord, TypeOp, UnknownTypeOp};
use error::TxProError;

/// Column indices resolved from the header row.
pub struct Columns {
    type_op: usize,
    client: usize,
    tx: usize,
    amount: usize,
}

impl Columns {
    pub fn resolve(headers: &StringRecord) -> Result<Self, TxProError> {
        // Helper: find a column by name or report it missing.
        let find = |name: &str| -> Result<usize, TxProError> {
            headers
                .iter()
                .position(|h| h.eq_ignore_ascii_case(name))
                .ok_or_else(|| TxProError::MissingColumn(name.to_string()))
        };

        Ok(Self {
            type_op: find("type")?,
            client: find("client")?,
            tx: find("tx")?,
            amount: find("amount")?,
        })
    }
}

pub struct InputRecordParser;

impl InputRecordParser {
    pub fn parse_record(
        record: &StringRecord,
        cols: &Columns,
        line: u64,
    ) -> Result<(InputRecord, TypeOp), TxProError> {
        let get = |idx: usize, name: &'static str| -> Result<&str, TxProError> {
            record.get(idx).ok_or_else(|| TxProError::BadField {
                line,
                column: name,
                value: "<missing>".into(),
                expected: "present",
            })
        };

        let type_op_str = get(cols.type_op, "type")?.to_string();
        let type_op: TypeOp =
            type_op_str
                .parse()
                .map_err(|source: UnknownTypeOp| TxProError::BadField {
                    line,
                    column: "type",
                    value: source.0,
                    expected: "TypeOp",
                })?;

        let client_raw = get(cols.client, "client")?;
        let client: u16 = client_raw.parse().map_err(|_| TxProError::BadField {
            line,
            column: "client",
            value: client_raw.to_string(),
            expected: "u16",
        })?;

        let tx_raw = get(cols.tx, "tx")?;
        let tx: u32 = tx_raw.parse().map_err(|_| TxProError::BadField {
            line,
            column: "tx",
            value: tx_raw.to_string(),
            expected: "u32",
        })?;

        let amount_raw = get(cols.amount, "amount")?;
        let amount: Decimal = if amount_raw.is_empty() {
            Decimal::ZERO
        } else {
            amount_raw.parse().map_err(|_| TxProError::BadField {
                line,
                column: "amount",
                value: amount_raw.to_string(),
                expected: "Decimal",
            })?
        };

        let amount = amount.trunc_with_scale(4);

        Ok((
            InputRecord {
                type_op: type_op_str,
                client,
                tx,
                amount,
            },
            type_op,
        ))
    }
}
