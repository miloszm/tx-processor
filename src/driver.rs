use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use csv::{ReaderBuilder, StringRecord};

use crate::accounts::Accounts;
use crate::dispatcher::dispatch;
use crate::error::TxProError;
use crate::parser::{Columns, InputRecordParser};
use crate::types::OpOutcome;

pub struct TxProcessor;

impl TxProcessor {
    pub fn run(file_path: impl AsRef<Path>) -> Result<(), TxProError> {
        let file = File::open(&file_path)?;

        let stdout = std::io::stdout();
        let mut handle = stdout.lock();
        let stderr = std::io::stderr();
        let mut handle_err = stderr.lock();
        Self::process(file, &mut handle, &mut handle_err)
    }

    pub fn process<R: Read, W: Write, E: Write>(
        input: R,
        out: &mut W,
        err: &mut E,
    ) -> Result<(), TxProError> {
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .trim(csv::Trim::All) // trims whitespace around fields
            .from_reader(input);

        let headers = reader.headers()?.clone();
        let cols = Columns::resolve(&headers)?;

        let mut accounts = Accounts::new();

        let mut record = StringRecord::new();

        while reader.read_record(&mut record)? {
            let line_no = reader.position().line();
            let input_record = InputRecordParser::parse_record(&record, &cols, line_no)?;
            match dispatch(&input_record, &mut accounts)? {
                OpOutcome::Applied => {}
                OpOutcome::Rejected(reason) => {
                    writeln!(err, "tx {}: rejected: {:?}", input_record.tx, reason)?;
                }
            }
        }

        accounts.print_accounts(out)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn end_to_end() {
        let input = "\
            type,client,tx,amount\n\
            deposit,1,100,10.00\n\
            withdrawal,1,101,3.50\n";
        let mut out = Vec::new();
        let mut err = Vec::new();

        TxProcessor::process(input.as_bytes(), &mut out, &mut err).unwrap();

        let out = String::from_utf8(out).unwrap();
        assert_eq!(
            out,
            "client,available,held,total,locked\n\
            1,6.5000,0,6.5000,false\n"
        );
    }

    #[test]
    fn end_to_end_with_dispute_and_chargeback() {
        let input = "\
        type,client,tx,amount\n\
        deposit,1,100,10.00\n\
        dispute,1,100,\n\
        chargeback,1,100,\n";
        let mut out = Vec::new();
        let mut err = Vec::new();

        TxProcessor::process(input.as_bytes(), &mut out, &mut err).unwrap();

        let out = String::from_utf8(out).unwrap();
        assert_eq!(
            out,
            "client,available,held,total,locked\n\
             1,0.0000,0.0000,0.0000,true\n"
        );
    }
}
