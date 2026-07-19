use std::collections::HashMap;
use std::io::{self, BufRead};

use super::transaction::{
    Transaction, TransactionStatus, TransactionType, format_quoted_description,
    parse_quoted_description,
};
use crate::parsers::{FormatDetector, TransactionParser, TransactionSerializer};
use crate::{Error, Result};

pub struct TxtParser;
pub struct TxtSerializer;

#[derive(PartialEq)]
enum State {
    Collect,
    Save,
    Finish,
}

pub struct TxtIterator {
    lines: io::Lines<Box<dyn io::BufRead>>,
    state: State,
    current_record: HashMap<String, String>,
}

impl TxtIterator {
    pub fn new(reader: Box<dyn io::BufRead>) -> Self {
        TxtIterator {
            lines: reader.lines(),
            state: State::Collect,
            current_record: HashMap::new(),
        }
    }

    fn make_transaction(&self) -> Result<Transaction> {
        // Извлекаем поля с проверкой наличия
        let tx_id_str = self
            .current_record
            .get("TX_ID")
            .ok_or_else(|| Error::ParseError("Missing TX_ID field".to_string()))?;

        let tx_type_str = self
            .current_record
            .get("TX_TYPE")
            .ok_or_else(|| Error::ParseError("Missing TX_TYPE field".to_string()))?;

        let from_user_id_str = self
            .current_record
            .get("FROM_USER_ID")
            .ok_or_else(|| Error::ParseError("Missing FROM_USER_ID field".to_string()))?;

        let to_user_id_str = self
            .current_record
            .get("TO_USER_ID")
            .ok_or_else(|| Error::ParseError("Missing TO_USER_ID field".to_string()))?;

        let amount_str = self
            .current_record
            .get("AMOUNT")
            .ok_or_else(|| Error::ParseError("Missing AMOUNT field".to_string()))?;

        let timestamp_str = self
            .current_record
            .get("TIMESTAMP")
            .ok_or_else(|| Error::ParseError("Missing TIMESTAMP field".to_string()))?;

        let status_str = self
            .current_record
            .get("STATUS")
            .ok_or_else(|| Error::ParseError("Missing STATUS field".to_string()))?;

        let description = parse_quoted_description(
            self.current_record
                .get("DESCRIPTION")
                .ok_or_else(|| Error::ParseError("Missing DESCRIPTION field".to_string()))?,
        )?;

        // Парсим значения
        let tx_id: u64 = tx_id_str
            .parse()
            .map_err(|e| Error::ParseError(format!("Invalid TX_ID '{}': {}", tx_id_str, e)))?;

        let tx_type = TransactionType::from_str(tx_type_str)?;

        let from_user_id: u64 = from_user_id_str.parse().map_err(|e| {
            Error::ParseError(format!(
                "Invalid FROM_USER_ID '{}': {}",
                from_user_id_str, e
            ))
        })?;

        let to_user_id: u64 = to_user_id_str.parse().map_err(|e| {
            Error::ParseError(format!("Invalid TO_USER_ID '{}': {}", to_user_id_str, e))
        })?;

        let amount: u64 = amount_str
            .parse()
            .map_err(|e| Error::ParseError(format!("Invalid AMOUNT '{}': {}", amount_str, e)))?;

        let timestamp: u64 = timestamp_str.parse().map_err(|e| {
            Error::ParseError(format!("Invalid TIMESTAMP '{}': {}", timestamp_str, e))
        })?;

        let status = TransactionStatus::from_str(status_str)?;

        Ok(Transaction::new(
            tx_id,
            tx_type,
            from_user_id,
            to_user_id,
            amount,
            timestamp,
            status,
            description,
        ))
    }

    fn parse_line(&mut self, line: &str) -> Result<()> {
        let Some((fld_name, fld_value)) = line.split_once(':') else {
            return Err(Error::ParseError(format!(
                "Invalid line (expected KEY: VALUE): {}",
                line
            )));
        };
        let fld_name = fld_name.trim().to_string();
        if self.current_record.contains_key(&fld_name) {
            return Err(Error::ParseError(format!(
                "Duplicate field '{}'",
                fld_name
            )));
        }
        self.current_record
            .insert(fld_name, fld_value.trim().to_string());
        Ok(())
    }
}

impl Iterator for TxtIterator {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                State::Collect => {
                    let nxt_line = self.lines.next();

                    if let Some(line) = nxt_line {
                        let line =
                            line.map_err(|e| Error::make_sys_error(Box::new(e), "TxtParser"));
                        match line {
                            Err(e) => break Some(Err(e)),
                            Ok(line) => {
                                if line.trim().is_empty() {
                                    self.state = State::Save;
                                    continue;
                                }

                                if line.trim_start().starts_with('#') {
                                    continue;
                                }

                                if let Err(e) = self.parse_line(line.as_str()) {
                                    self.state = State::Finish;
                                    break Some(Err(e));
                                }
                            }
                        }
                    } else {
                        // EOF: emit pending record if any, otherwise finish cleanly
                        self.state = State::Finish;
                        if self.current_record.is_empty() {
                            break None;
                        }
                        break Some(self.make_transaction());
                    }
                }
                State::Save => {
                    // Blank lines with no accumulated fields are separators / trailing noise
                    if self.current_record.is_empty() {
                        self.state = State::Collect;
                        continue;
                    }
                    let tx = self.make_transaction();
                    self.state = State::Collect;
                    self.current_record.clear();
                    break Some(tx);
                }
                State::Finish => break None,
            }
        }
    }
}

impl FormatDetector for TxtParser {
    fn detect(buffer: &[u8]) -> bool {
        // See: YPBankTextFormat_ru.md
        let text = match std::str::from_utf8(buffer) {
            Ok(s) => s,
            Err(_) => return false,
        };

        for row in text.lines() {
            let trimmed = row.trim();
            if trimmed.is_empty() || trimmed.starts_with("#") {
                continue;
            }

            return trimmed.starts_with("TX_ID:")
                || trimmed.starts_with("TX_TYPE:")
                || trimmed.starts_with("FROM_USER_ID:")
                || trimmed.starts_with("TO_USER_ID:")
                || trimmed.starts_with("AMOUNT:")
                || trimmed.starts_with("TIMESTAMP:")
                || trimmed.starts_with("STATUS:")
                || trimmed.starts_with("DESCRIPTION:");
        }

        false
    }
}

impl TransactionParser for TxtParser {
    type Iter = TxtIterator;

    fn parse(&self, reader: Box<dyn io::BufRead>) -> Result<Self::Iter> {
        Ok(TxtIterator::new(reader))
    }
}

impl TransactionSerializer for TxtSerializer {
    fn serialize(
        &self,
        writer: &mut dyn io::Write,
        transactions: &mut dyn Iterator<Item = Result<Transaction>>,
    ) -> Result<()> {
        let mut save_empty_line: bool = false;
        for tx in transactions {
            if save_empty_line {
                writeln!(writer, "")
                    .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?; // Empty line between records.
            } else {
                save_empty_line = true;
            }
            let tx = tx.map_err(|e| e)?;
            writeln!(writer, "TX_ID: {}", tx.id)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "TX_TYPE: {}", tx.operation)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "FROM_USER_ID: {}", tx.from_user)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "TO_USER_ID: {}", tx.to_user)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "AMOUNT: {}", tx.amount)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "TIMESTAMP: {}", tx.timestamp)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "STATUS: {}", tx.status)
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
            writeln!(writer, "DESCRIPTION: {}", format_quoted_description(&tx.description))
                .map_err(|e| Error::make_sys_error(Box::new(e), "TxtSerializer"))?;
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
    fn test_parse_valid_text() {
        let data = r#"# Record 1 (Deposit)
    TX_ID: 1234567890123456
    TX_TYPE: DEPOSIT
    FROM_USER_ID: 0
    TO_USER_ID: 9876543210987654
    AMOUNT: 10000
    TIMESTAMP: 1633036800000
    STATUS: SUCCESS
    DESCRIPTION: "Terminal deposit"

    # Record 2 (Transfer)
    TX_ID: 2312321321321321
    TIMESTAMP: 1633056800000
    STATUS: FAILURE
    TX_TYPE: TRANSFER
    FROM_USER_ID: 1231231231231231
    TO_USER_ID: 9876543210987654
    AMOUNT: 1000
    DESCRIPTION: "User transfer"

    # Record 3 (Withdrawal)
    TX_ID: 3213213213213213
    AMOUNT: 100
    TX_TYPE: WITHDRAWAL
    FROM_USER_ID: 9876543210987654
    TO_USER_ID: 0
    TIMESTAMP: 1633066800000
    STATUS: SUCCESS
    DESCRIPTION: "User withdrawal"
    "#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let tx1 = iter.next().unwrap().unwrap();
        assert_eq!(tx1.id, 1234567890123456);
        assert_eq!(tx1.operation, TransactionType::Deposit);
        assert_eq!(tx1.from_user, 0);
        assert_eq!(tx1.to_user, 9876543210987654);
        assert_eq!(tx1.amount, 10000);
        assert_eq!(tx1.timestamp, 1633036800000);
        assert_eq!(tx1.status, TransactionStatus::Success);
        assert_eq!(tx1.description, "Terminal deposit");

        let tx2 = iter.next().unwrap().unwrap();
        assert_eq!(tx2.id, 2312321321321321);
        assert_eq!(tx2.operation, TransactionType::Transfer);
        assert_eq!(tx2.from_user, 1231231231231231);
        assert_eq!(tx2.to_user, 9876543210987654);
        assert_eq!(tx2.amount, 1000);
        assert_eq!(tx2.timestamp, 1633056800000);
        assert_eq!(tx2.status, TransactionStatus::Failure);
        assert_eq!(tx2.description, "User transfer");

        let tx3 = iter.next().unwrap().unwrap();
        assert_eq!(tx3.id, 3213213213213213);
        assert_eq!(tx3.operation, TransactionType::Withdrawal);
        assert_eq!(tx3.from_user, 9876543210987654);
        assert_eq!(tx3.to_user, 0);
        assert_eq!(tx3.amount, 100);
        assert_eq!(tx3.timestamp, 1633066800000);
        assert_eq!(tx3.status, TransactionStatus::Success);
        assert_eq!(tx3.description, "User withdrawal");

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_parse_text_missing_field() {
        let data = r#"TX_ID: 1
    TX_TYPE: DEPOSIT
    FROM_USER_ID: 0
    TO_USER_ID: 1001
    AMOUNT: 100
    TIMESTAMP: 1633036800000
    # STATUS missing!
    DESCRIPTION: "Test"
    "#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let result = iter.next().unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Missing STATUS"));
    }

    #[test]
    fn test_parse_text_invalid_field() {
        let data = r#"TX_ID: not_a_number
    TX_TYPE: DEPOSIT
    FROM_USER_ID: 0
    TO_USER_ID: 1001
    AMOUNT: 100
    TIMESTAMP: 1633036800000
    STATUS: SUCCESS
    DESCRIPTION: "Test"
    "#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let result = iter.next().unwrap();
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid TX_ID"));
    }

    #[test]
    fn test_parse_text_duplicate_field() {
        let data = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1001
AMOUNT: 100
AMOUNT: 200
TIMESTAMP: 1633036800000
STATUS: SUCCESS
DESCRIPTION: "Test"
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let result = iter.next().unwrap();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Duplicate field 'AMOUNT'")
        );
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_parse_empty_file() {
        let cursor = Box::new(BufReader::new(Cursor::new("")));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_parse_comments_only() {
        let data = "# just a comment\n# another\n";
        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_parse_trailing_blank_lines() {
        let data = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1001
AMOUNT: 100
TIMESTAMP: 1633036800000
STATUS: SUCCESS
DESCRIPTION: "Test"


"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.id, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_parse_record_without_trailing_blank_line() {
        let data = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1001
AMOUNT: 100
TIMESTAMP: 1633036800000
STATUS: SUCCESS
DESCRIPTION: "Test""#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let tx = iter.next().unwrap().unwrap();
        assert_eq!(tx.id, 1);
        assert!(iter.next().is_none());
    }

    #[test]
    fn test_write_text() {
        let tx1 = Transaction::new(
            1234567890123456,
            TransactionType::Deposit,
            0,
            9876543210987654,
            10000,
            1633036800000,
            TransactionStatus::Success,
            "Terminal deposit".to_string(),
        );

        let tx2 = Transaction::new(
            2312321321321321,
            TransactionType::Transfer,
            1231231231231231,
            9876543210987654,
            1000,
            1633056800000,
            TransactionStatus::Failure,
            "User transfer".to_string(),
        );

        let mut output = Vec::new();
        let transactions = vec![Ok(tx1), Ok(tx2)];

        let mut iter = transactions.into_iter();
        let iter_ref: &mut dyn Iterator<Item = Result<Transaction>> = &mut iter;

        let serializer = TxtSerializer;
        serializer.serialize(&mut output, iter_ref).unwrap();

        let result = String::from_utf8(output).unwrap();
        let expected = r#"TX_ID: 1234567890123456
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 9876543210987654
AMOUNT: 10000
TIMESTAMP: 1633036800000
STATUS: SUCCESS
DESCRIPTION: "Terminal deposit"

TX_ID: 2312321321321321
TX_TYPE: TRANSFER
FROM_USER_ID: 1231231231231231
TO_USER_ID: 9876543210987654
AMOUNT: 1000
TIMESTAMP: 1633056800000
STATUS: FAILURE
DESCRIPTION: "User transfer"
"#;

        assert_eq!(result, expected);
    }

    #[test]
    fn test_parse_text_unquoted_description() {
        let data = r#"TX_ID: 1
TX_TYPE: DEPOSIT
FROM_USER_ID: 0
TO_USER_ID: 1001
AMOUNT: 100
TIMESTAMP: 1633036800000
STATUS: SUCCESS
DESCRIPTION: Test without quotes
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let result = iter.next().unwrap();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("DESCRIPTION must be enclosed in double quotes")
        );
    }

    #[test]
    fn test_parse_text_line_without_colon() {
        let data = r#"TX_ID: 1
not a field line
TX_TYPE: DEPOSIT
"#;

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = TxtParser;
        let mut iter = parser.parse(cursor).unwrap();

        let result = iter.next().unwrap();
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("expected KEY: VALUE")
        );
    }

    #[test]
    fn test_txt_roundtrip_description_with_quotes() {
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
        TxtSerializer
            .serialize(&mut output, iter_ref)
            .unwrap();

        let cursor = Box::new(BufReader::new(Cursor::new(output)));
        let parsed = TxtParser.parse(cursor).unwrap().next().unwrap().unwrap();
        assert_eq!(parsed.description, tx.description);
        assert!(parsed.diff(&tx).is_none());
    }

    #[test]
    fn test_detect_text_format() {
        // Текстовый формат с ключами
        let data = b"TX_ID: 1\nTX_TYPE: DEPOSIT\n";
        assert!(TxtParser::detect(data));

        // Текстовый формат с комментарием
        let data = b"# Comment\nTX_ID: 1\n";
        assert!(TxtParser::detect(data));

        // Не текстовый формат
        let data = b"Some random text without colon";
        assert!(!TxtParser::detect(data));

        // Пустой буфер
        let data = b"";
        assert!(!TxtParser::detect(data));
    }
}
