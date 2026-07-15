use std::io::{self, BufRead, BufReader, Read, Write};

use super::transaction::{Transaction, TransactionStatus, TransactionType};
use crate::error::{Error, Result};
use crate::parsers::{FormatDetector, TransactionReader, TransactionWriter};

const CSV_HEADER: &str =
    "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";

pub struct CsvParser;

pub struct CsvIterator {
    reader: io::Lines<BufReader<Box<dyn Read>>>,
}

impl CsvIterator {
    pub fn new(reader: Box<dyn Read>) -> Self {
        CsvIterator {
            reader: BufReader::new(reader).lines(),
        }
    }

    pub fn parse_line(line: &str) -> Result<Transaction> {
        Err(Error::InvalidData("FUCK".to_string()))
    }
}

impl Iterator for CsvIterator {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = match self.reader.next() {
                Some(Ok(line)) => line,
                Some(Err(e)) => return Some(Err(Error::make_io_error(e, "UPS"))),
                None => return None,
            };

            if line.trim().is_empty() {
                continue;
            }

            return Some(Self::parse_line(&line));
        }
    }
}

impl FormatDetector for CsvParser {
    fn detect(buffer: &[u8]) -> bool {
        // First Line always is TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
        // See: YPBankCsvFormat_ru.md
        let text = match std::str::from_utf8(buffer) {
            Ok(s) => s,
            Err(_) => return false,
        };

        text.starts_with(CSV_HEADER)
    }
}

impl TransactionReader for CsvParser {
    type Iter = CsvIterator;

    fn read_transactions<R: Read + 'static>(&self, reader: R) -> Result<Self::Iter> {
        Ok(CsvIterator::new(Box::new(reader)))
    }
}

impl TransactionWriter for CsvParser {
    fn write_transactions<W: Write, I: Iterator<Item = Result<Transaction>>>(
        &self,
        mut writer: W,
        transactions: I,
    ) -> Result<()> {
        Ok(())
    }
}
