use std::io::{self, BufRead};

use super::transaction::{Transaction, TransactionStatus, TransactionType};
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

impl CsvIterator {
    pub fn new(reader: Box<dyn io::BufRead>) -> Self {
        CsvIterator {
            lines: reader.lines(),
            header_processed: false,
        }
    }

    fn parse_line(&self, line: &str) -> Result<Transaction> {
        let mut fields = Vec::<String>::new();
        let mut current_field = String::new();
        let mut counter = 0;

        let mut chars = line.chars().peekable();
        while let Some(ch) = chars.next() {
            if counter > FIELDS_COUNT - 1 {
                break;
            }
            match ch {
                '"' => {
                    current_field.push(ch);
                    while let Some(ch) = chars.next() {
                        current_field.push(ch);
                        if ch == '"' {
                            if let Some(&nxt_ch) = chars.peek() {
                                if nxt_ch == ',' {
                                    break;
                                }
                            }
                        }
                    }
                }
                ',' => {
                    fields.push(current_field.clone());
                    current_field.clear();
                    counter += 1;
                }
                _ => current_field.push(ch),
            }
        }

        fields.push(current_field);
        counter += 1;

        if counter != FIELDS_COUNT {
            return Err(Error::ParseError(format!(
                "Expected 8 fields, got {} in line: {}",
                counter, line
            )));
        }

        let tx_id: u64 = fields[0]
            .trim()
            .parse()
            .map_err(|e| Error::ParseError(format!("Invalid TX_ID '{}': {}", fields[0], e)))?;

        let tx_op = TransactionType::from_str(fields[1].trim())?;

        let tx_from_user: u64 = fields[2].trim().parse().map_err(|e| {
            Error::ParseError(format!("Invalid FROM_USER_ID '{}': {}", fields[2], e))
        })?;

        let tx_to_user: u64 = fields[3]
            .trim()
            .parse()
            .map_err(|e| Error::ParseError(format!("Invalid TO_USER_ID '{}': {}", fields[3], e)))?;

        let tx_amount: u64 = fields[4]
            .trim()
            .parse()
            .map_err(|e| Error::ParseError(format!("Invalid AMOUNT '{}': {}", fields[4], e)))?;

        let tx_timestamp: u64 = fields[5]
            .trim()
            .parse()
            .map_err(|e| Error::ParseError(format!("Invalid TIMESTAMP '{}': {}", fields[5], e)))?;

        let tx_status = TransactionStatus::from_str(fields[6].trim())?;

        let tx_description = fields[7].trim();

        // println!("{:?}", fields);
        Ok(Transaction::new(
            tx_id,
            tx_op,
            tx_from_user,
            tx_to_user,
            tx_amount,
            tx_timestamp,
            tx_status,
            tx_description.to_string(),
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
                            let e = Err(Error::ParseError(format!("Csv header is missing!")));
                            break Some(e);
                        }

                        self.header_processed = true;
                        continue;
                    }

                    if line.is_empty() {
                        continue;
                    }

                    let parse_result = self.parse_line(line.as_str());
                    break Some(parse_result);
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
        // Put Header First
        writeln!(writer, "{}", CSV_HEADER)
            .map_err(|e| Error::make_sys_error(Box::new(e), "CsvSerializer"))?;
        // Save Trabsactions
        for tx in transactions {
            let tx = tx.map_err(|e| e)?;
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
                tx.description
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
    fn test_parse_valid_csv() {
        let data = r#"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION
1,DEPOSIT,0,1001,50000,1672531200000,SUCCESS,"Initial deposit"
2,TRANSFER,1001,1002,15000,1672534800000,FAILURE,"Payment for services"
3,WITHDRAWAL,1002,0,1000,1672538400000,PENDING,"ATM withdrawal"
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = CsvParser;
        let mut iter = parser.parse(cursor).unwrap();

        // Первая транзакция
        let tx1 = iter.next().unwrap().unwrap();
        assert_eq!(tx1.id, 1);
        assert_eq!(tx1.operation, TransactionType::Deposit);
        assert_eq!(tx1.from_user, 0);
        assert_eq!(tx1.to_user, 1001);
        assert_eq!(tx1.amount, 50000);
        assert_eq!(tx1.timestamp, 1672531200000);
        assert_eq!(tx1.status, TransactionStatus::Success);
        assert_eq!(tx1.description, "\"Initial deposit\"");

        // Вторая транзакция
        let tx2 = iter.next().unwrap().unwrap();
        assert_eq!(tx2.id, 2);
        assert_eq!(tx2.operation, TransactionType::Transfer);
        assert_eq!(tx2.from_user, 1001);
        assert_eq!(tx2.to_user, 1002);
        assert_eq!(tx2.amount, 15000);
        assert_eq!(tx2.timestamp, 1672534800000);
        assert_eq!(tx2.status, TransactionStatus::Failure);
        assert_eq!(tx2.description, "\"Payment for services\"");

        // Третья транзакция
        let tx3 = iter.next().unwrap().unwrap();
        assert_eq!(tx3.id, 3);
        assert_eq!(tx3.operation, TransactionType::Withdrawal);
        assert_eq!(tx3.from_user, 1002);
        assert_eq!(tx3.to_user, 0);
        assert_eq!(tx3.amount, 1000);
        assert_eq!(tx3.timestamp, 1672538400000);
        assert_eq!(tx3.status, TransactionStatus::Pending);
        assert_eq!(tx3.description, "\"ATM withdrawal\"");

        // Больше не должно быть
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
        assert_eq!(tx.description, r#""Description with ""quotes"" inside""#);
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
            "\"Initial deposit\"".to_string(),
        );

        let tx2 = Transaction::new(
            2,
            TransactionType::Transfer,
            1001,
            1002,
            15000,
            1672534800000,
            TransactionStatus::Failure,
            r#""Payment with "quotes"""#.to_string(),
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
2,TRANSFER,1001,1002,15000,1672534800000,FAILURE,"Payment with "quotes""
"#;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_detect_csv_format() {
        // CSV с правильным заголовком
        let data = b"TX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION";
        assert!(CsvParser::detect(data));

        // Не CSV
        let data = b"Some random text";
        assert!(!CsvParser::detect(data));

        // Пустой буфер
        let data = b"";
        assert!(!CsvParser::detect(data));

        // Неправильный заголовок
        let data = b"ID,TYPE,FROM,TO,AMOUNT,TS,STATUS,DESC";
        assert!(!CsvParser::detect(data));
    }
}
