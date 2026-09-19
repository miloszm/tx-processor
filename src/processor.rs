use std::error::Error;
use std::fs::File;
use std::path::Path;

use csv::ReaderBuilder;
use crate::error::TxProError;

pub struct TxProcessor;

impl TxProcessor {
    pub fn process(file_path: impl AsRef<Path>) -> Result<(), TxProError>{
        let file = File::open(&file_path)?;

        // Build the reader. `has_headers(true)` is the default and will skip
        // the first row when iterating over records().
        let mut reader = ReaderBuilder::new()
            .has_headers(true)
            .from_reader(file);

        // Optional: print the header row once, up front.
        if let Some(headers) = Some(reader.headers()?) {
            println!("Headers: {:?}", headers);
        }

        // `records()` returns an iterator. Each `next()` pulls in exactly one
        // record from the underlying buffered reader.
        let mut line_no = 0usize;
        for result in reader.records() {
            let record = result?; // a csv::StringRecord
            line_no += 1;

            // Access fields by index or iterate over them.
            // Fields are not copied unless you ask for them.
            println!(
                "line {}: fields={:?}",
                line_no,
                record.iter().collect::<Vec<_>>()
            );

            // You could also do:
            // let name = record.get(0).unwrap_or("");
            // let age: u32 = record.get(1).unwrap_or("0").parse()?;
        }

        Ok(())
    }
}
