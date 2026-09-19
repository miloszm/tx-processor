use std::fs::File;
use std::path::Path;

use crate::accounts::Accounts;
use crate::error::TxProError;
use crate::parser::{Columns, InputRecordParser};
use crate::processor::InputRecordProcessor;
use csv::{ReaderBuilder, StringRecord};

pub struct TxProcessor;

impl TxProcessor {
    pub fn run(file_path: impl AsRef<Path>) -> Result<(), TxProError> {
        let file = File::open(&file_path)?;

        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .trim(csv::Trim::All) // trims whitespace around fields
            .from_reader(file);

        let headers = reader.headers()?.clone();
        let cols = Columns::resolve(&headers)?;
        let mut line_no = 1u64;

        let mut accounts = Accounts::new();

        let mut record = StringRecord::new();

        while reader.read_record(&mut record)? {
            line_no += 1;
            let input_record = InputRecordParser::parse_record(&record, &cols, line_no)?;
            let _op_outcome = InputRecordProcessor::process(input_record, &mut accounts)?;
        }

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        accounts.print_accounts(&mut handle)?;

        Ok(())
    }
}
