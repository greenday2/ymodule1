use std::io::{self, BufRead};

use super::transaction::{
    Transaction, TransactionStatus, TransactionType, format_quoted_description,
};
use crate::parsers::{FormatDetector, TransactionParser, TransactionSerializer};
use crate::{Error, Result};

const FIELDS_COUNT: usize = 8;
const CSV_HEADER: &str =
    "TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";

pub struct CsvParser;
pub struct CsvSerializer;

pub struct CsvIterator {
    lines: io::Lines<Box<dyn io::BufRead>>,
    header_processed: bool,
}

#[derive(Debug, PartialEq)]
struct CsvField {
    value: String,
    quoted: bool,
}

/// Split a CSV line into fields. Quoted fields may contain commas; `""` → `"`.
fn split_csv_line(line: &str) -> Result<Vec<CsvField>> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut in_quotes = false;
    let mut field_quoted = false;

    while let Some(ch) = chars.next() {
        match ch {
            '"' => {
                if in_quotes {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        current.push('"');
                    } else {
                        in_quotes = false;
                    }
                } else if current.is_empty() {
                    in_quotes = true;
                    field_quoted = true;
                } else {
                    return Err(Error::ParseError(format!(
                        "Unexpected quote in CSV field at line: {}",
                        line
                    )));
                }
            }
            ',' if !in_quotes => {
                fields.push(CsvField {
                    value: std::mem::take(&mut current),
                    quoted: field_quoted,
                });
                field_quoted = false;
            }
            _ => current.push(ch),
        }
    }

    if in_quotes {
        return Err(Error::ParseError(format!(
            "Unclosed quote in CSV line: {}",
            line
        )));
    }

    fields.push(CsvField {
        value: current,
        quoted: field_quoted,
    });

    if fields.len() != FIELDS_COUNT {
        return Err(Error::ParseError(format!(
            "Expected {} fields, got {} in line: {}",
            FIELDS_COUNT,
            fields.len(),
            line
        )));
    }

    Ok(fields)
}

impl CsvIterator {
    pub fn new(reader: Box<dyn io::BufRead>) -> Self {
        CsvIterator {
            lines: reader.lines(),
            header_processed: false,
        }
    }

    fn parse_line(&self, line: &str) -> Result<Transaction> {
        let fields = split_csv_line(line)?;

        let tx_id: u64 = fields[0].value.trim().parse().map_err(|e| {
            Error::ParseError(format!("Invalid TX_ID '{}': {}", fields[0].value, e))
        })?;

        let tx_op = TransactionType::from_str(fields[1].value.trim())?;

        let tx_from_user: u64 = fields[2].value.trim().parse().map_err(|e| {
            Error::ParseError(format!(
                "Invalid FROM_USER_ID '{}': {}",
                fields[2].value, e
            ))
        })?;

        let tx_to_user: u64 = fields[3].value.trim().parse().map_err(|e| {
            Error::ParseError(format!("Invalid TO_USER_ID '{}': {}", fields[3].value, e))
        })?;

        let tx_amount: u64 = fields[4].value.trim().parse().map_err(|e| {
            Error::ParseError(format!("Invalid AMOUNT '{}': {}", fields[4].value, e))
        })?;

        let tx_timestamp: u64 = fields[5].value.trim().parse().map_err(|e| {
            Error::ParseError(format!(
                "Invalid TIMESTAMP '{}': {}",
                fields[5].value, e
            ))
        })?;

        let tx_status = TransactionStatus::from_str(fields[6].value.trim())?;

        if !fields[7].quoted {
            return Err(Error::ParseError(format!(
                "DESCRIPTION must be enclosed in double quotes, got: {}",
                fields[7].value
            )));
        }
        let tx_description = fields[7].value.clone();

        Ok(Transaction::new(
            tx_id,
            tx_op,
            tx_from_user,
            tx_to_user,
            tx_amount,
            tx_timestamp,
            tx_status,
            tx_description,
        ))
    }
}

impl Iterator for CsvIterator {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let line = self
                .lines
                .next()?
                .map_err(|e| Error::make_sys_error(Box::new(e), "CsvParser"));

            match line {
                Err(e) => break Some(Err(e)),
                Ok(line) => {
                    if !self.header_processed {
                        if line != CSV_HEADER {
                            break Some(Err(Error::ParseError(
                                "Csv header is missing!".to_string(),
                            )));
                        }

                        self.header_processed = true;
                        continue;
                    }

                    if line.is_empty() {
                        continue;
                    }

                    break Some(self.parse_line(line.as_str()));
                }
            }
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

impl TransactionParser for CsvParser {
    type Iter = CsvIterator;

    fn parse(&self, reader: Box<dyn io::BufRead>) -> Result<Self::Iter> {
        Ok(CsvIterator::new(reader))
    }
}

impl TransactionSerializer for CsvSerializer {
    fn serialize(
        &self,
        writer: &mut dyn io::Write,
        transactions: &mut dyn Iterator<Item = Result<Transaction>>,
    ) -> Result<()> {
        writeln!(writer, "{}", CSV_HEADER)
            .map_err(|e| Error::make_sys_error(Box::new(e), "CsvSerializer"))?;
        for tx in transactions {
            let tx = tx?;
            writeln!(
                writer,
                "{},{},{},{},{},{},{},{}",
                tx.id,
                tx.operation,
                tx.from_user,
                tx.to_user,
                tx.amount,
                tx.timestamp,
                tx.status,
                format_quoted_description(&tx.description)
            )
            .map_err(|e| Error::make_sys_error(Box::new(e), "CsvSerializer"))?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::BufReader;
    use std::io::Cursor;

    #[test]
    fn test_split_csv_line_basic() {
        let fields = split_csv_line(
            r#"1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Initial deposit""#,
        )
        .unwrap();
        assert_eq!(fields.len(), 8);
        assert!(!fields[0].quoted);
        assert_eq!(fields[0].value, "1");
        assert!(fields[7].quoted);
        assert_eq!(fields[7].value, "Initial deposit");
    }

    #[test]
    fn test_split_csv_line_comma_inside_quotes() {
        let fields = split_csv_line(
            r#"1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Payment for services, invoice #123""#,
        )
        .unwrap();
        assert_eq!(fields[7].value, "Payment for services, invoice #123");
    }

    #[test]
    fn test_split_csv_line_escaped_quotes() {
        let fields = split_csv_line(
            r#"1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Description with ""quotes"" inside""#,
        )
        .unwrap();
        assert_eq!(fields[7].value, r#"Description with "quotes" inside"#);
    }

    #[test]
    fn test_split_csv_line_unclosed_quote() {
        let err = split_csv_line(r#"1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"oops"#)
            .unwrap_err();
        assert!(err.to_string().contains("Unclosed quote"));
    }

    #[test]
    fn test_split_csv_line_wrong_field_count() {
        let err = split_csv_line("1,2,3").unwrap_err();
        assert!(err.to_string().contains("Expected 8 fields"));
    }

    #[test]
    fn test_parse_valid_csv() {
        let data = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Initial deposit"
2,TRANSFER,1001,1002,15000,1672534800000,FAILURE,"Payment for services"
3,WITHDRAWAL,1002,0,1000,1672538400000,PENDING,"ATM withdrawal"
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = CsvParser;
        let mut iter = parser.parse(cursor).unwrap();

        let tx1 = iter.next().unwrap().unwrap();
        assert_eq!(tx1.id, 1);
        assert_eq!(tx1.operation, TransactionType::Deposit);
        assert_eq!(tx1.from_user, 0);
        assert_eq!(tx1.to_user, 1001);
        assert_eq!(tx1.amount, 50000);
        assert_eq!(tx1.timestamp, 1672531200000);
        assert_eq!(tx1.status, TransactionStatus::Success);
        assert_eq!(tx1.description, "Initial deposit");

        let tx2 = iter.next().unwrap().unwrap();
        assert_eq!(tx2.id, 2);
        assert_eq!(tx2.operation, TransactionType::Transfer);
        assert_eq!(tx2.from_user, 1001);
        assert_eq!(tx2.to_user, 1002);
        assert_eq!(tx2.amount, 15000);
        assert_eq!(tx2.timestamp, 1672534800000);
        assert_eq!(tx2.status, TransactionStatus::Failure);
        assert_eq!(tx2.description, "Payment for services");

        let tx3 = iter.next().unwrap().unwrap();
        assert_eq!(tx3.id, 3);
        assert_eq!(tx3.operation, TransactionType::Withdrawal);
        assert_eq!(tx3.from_user, 1002);
        assert_eq!(tx3.to_user, 0);
        assert_eq!(tx3.amount, 1000);
        assert_eq!(tx3.timestamp, 1672538400000);
        assert_eq!(tx3.status, TransactionStatus::Pending);
        assert_eq!(tx3.description, "ATM withdrawal");

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_parse_with_quotes_in_description() {
        let data = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Description with ""quotes"" inside"
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = CsvParser;
        let mut iter = parser.parse(cursor).unwrap();

        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.description, r#"Description with "quotes" inside"#);
    }

    #[test]
    fn test_parse_comma_in_description() {
        let data = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1001,DEPOSIT,0,501,50000,1672531200000,SUCCESS,"Payment for services, invoice #123"
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let mut iter = CsvParser.parse(cursor).unwrap();
        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.description, "Payment for services, invoice #123");
    }

    #[test]
    fn test_parse_unquoted_description() {
        let data = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,plain text
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let mut iter = CsvParser.parse(cursor).unwrap();
        let err = iter.next().unwrap().unwrap_err();
        assert!(err.to_string().contains("DESCRIPTION must be enclosed"));
    }

    #[test]
    fn test_parse_unclosed_quote() {
        let data = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"oops
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let mut iter = CsvParser.parse(cursor).unwrap();
        let err = iter.next().unwrap().unwrap_err();
        assert!(err.to_string().contains("Unclosed quote"));
    }

    #[test]
    fn test_write_csv() {
        let tx1 = Transaction::new(
            1,
            TransactionType::Deposit,
            0,
            1001,
            50000,
            1672531200000,
            TransactionStatus::Success,
            "Initial deposit".to_string(),
        );

        let tx2 = Transaction::new(
            2,
            TransactionType::Transfer,
            1001,
            1002,
            15000,
            1672534800000,
            TransactionStatus::Failure,
            r#"Payment with "quotes""#.to_string(),
        );

        let mut output = Vec::new();

        let transactions = vec![Ok(tx1), Ok(tx2)];
        let mut iter = transactions.into_iter();
        let iter_ref: &mut dyn Iterator<Item = Result<Transaction>> = &mut iter;

        let serializer = CsvSerializer;
        serializer.serialize(&mut output, iter_ref).unwrap();

        let result = String::from_utf8(output).unwrap();
        let expected = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Initial deposit"
2,TRANSFER,1001,1002,15000,1672534800000,FAILURE,"Payment with ""quotes"""
"#;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_csv_roundtrip_description_with_quotes() {
        let tx = Transaction::new(
            1,
            TransactionType::Deposit,
            0,
            1001,
            100,
            1,
            TransactionStatus::Success,
            r#"He said "hi""#.to_string(),
        );

        let mut output = Vec::new();
        let mut iter = vec![Ok(tx.clone())].into_iter();
        let iter_ref: &mut dyn Iterator<Item = Result<Transaction>> = &mut iter;
        CsvSerializer.serialize(&mut output, iter_ref).unwrap();

        let cursor = Box::new(BufReader::new(Cursor::new(output)));
        let parsed = CsvParser.parse(cursor).unwrap().next().unwrap().unwrap();
        assert_eq!(parsed.description, tx.description);
        assert!(parsed.diff(&tx).is_none());
    }

    #[test]
    fn test_detect_csv_format() {
        let data = b"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";
        assert!(CsvParser::detect(data));

        let data = b"Some random text";
        assert!(!CsvParser::detect(data));

        let data = b"";
        assert!(!CsvParser::detect(data));

        let data = b"ID,TYPE,FROM,TO,AMOUNT,TS,STATUS,DESC";
        assert!(!CsvParser::detect(data));
    }
}
