use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use crate::accounts::Accounts;
use crate::dispatcher::dispatch;
use crate::error::TxProError;
use crate::parser::{Columns, InputRecordParser};
use crate::types::OpOutcome;
use csv::{ReaderBuilder, StringRecord};

pub struct TxProcessor;

impl TxProcessor {
    pub fn run(file_path: impl AsRef<Path>) -> Result<(), TxProError> {
        let file = File::open(&file_path)?;

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        Self::process(file, &mut handle)
    }

    pub fn process<R: Read, W: Write>(input: R, out: &mut W) -> Result<(), TxProError> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .trim(csv::Trim::All) // trims whitespace around fields
            .from_reader(input);

        let headers = reader.headers()?.clone();
        let cols = Columns::resolve(&headers)?;
        let mut line_no = 1u64;

        let mut accounts = Accounts::new();

        let mut record = StringRecord::new();

        while reader.read_record(&mut record)? {
            line_no += 1;
            let input_record = InputRecordParser::parse_record(&record, &cols, line_no)?;
            match dispatch(&input_record, &mut accounts)? {
                OpOutcome::Applied => {}
                OpOutcome::Rejected(reason) => {
                    eprintln!("tx {}: rejected: {:?}", input_record.tx, reason);
                }
            }
        }

        accounts.print_accounts(out)?;

        Ok(())
    }
}

#[test]
fn end_to_end() {
    let input = "\
        type,client,tx,amount\n\
        deposit,1,100,10.00\n\
        withdrawal,1,101,3.50\n";
    let mut out = Vec::new();

    TxProcessor::process(input.as_bytes(), &mut out).unwrap();

    let out = String::from_utf8(out).unwrap();
    assert!(out.contains("1,6.5000,0,6.5000,false"));
}
