use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TxProError {
    /// Wrong number of command-line arguments.
    #[error("expected exactly one argument (a CSV file path), got {got}")]
    BadArgs { got: usize },
    #[error("failed to open `{path}`")]
    Io {
        path: PathBuf,
        #[source]
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
}
