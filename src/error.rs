use std::path::PathBuf;
use thiserror::Error;


#[derive(Debug, Error)]
pub enum TxProError {
    /// Wrong number of command-line arguments.
    #[error("expected exactly one argument (a CSV file path), got {got}")]
    BadArgs { got: usize },
    /// File couldn't be open or read.
    #[error("failed to read file: {source}")]
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
}
