use crate::types::UnknownTypeOp;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TxProError {
    #[error("failed to open `{path}`")]
    Open {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("I/O error")]
    Io {
        #[from]
        source: std::io::Error,
    },
    /// CSV reader/parser problem.
    #[error("CSV parse error: {source}")]
    Csv {
        #[from]
        source: csv::Error,
    },

    #[error("required column `{0}` missing from header")]
    MissingColumn(String),

    /// A column exists but its value couldn't be parsed into the target type.
    #[error("line {line}: column `{column}` = `{value}` is not a valid {expected}")]
    BadField {
        line: u64,
        column: &'static str,
        value: String,
        expected: &'static str,
    },

    /// The `type` column had a value that isn't a known operation.
    #[error("column `type` = `{value}` is not a valid operation")]
    BadTypeOp {
        value: String,
        #[source]
        source: UnknownTypeOp,
    },
}
