use std::io;

use super::transaction::{Transaction, TransactionStatus, TransactionType};
use crate::parsers::{FormatDetector, TransactionParser, TransactionSerializer};
use crate::{Error, Result};

pub struct BinParser;
pub struct BinSerializer;

pub struct BinIterator {
    reader: Box<dyn io::BufRead>,
    stop_parse: bool,
}

impl BinIterator {
    pub fn new(reader: Box<dyn io::BufRead>) -> Self {
        BinIterator {
            reader,
            stop_parse: false,
        }
    }

    fn read_bytes<'a>(buffer: &'a [u8], offset: &mut usize, len: usize) -> Result<&'a [u8]> {
        let end = offset
            .checked_add(len)
            .ok_or_else(|| Error::ParseError("Record offset overflow".to_string()))?;
        if end > buffer.len() {
            return Err(Error::ParseError(format!(
                "Truncated record: need {} bytes at offset {}, have {}",
                len,
                *offset,
                buffer.len()
            )));
        }
        let bytes = &buffer[*offset..end];
        *offset = end;
        Ok(bytes)
    }

    fn read_u32(buffer: &[u8], offset: &mut usize) -> Result<u32> {
        let bytes = Self::read_bytes(buffer, offset, 4)?;
        Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn read_u64(buffer: &[u8], offset: &mut usize) -> Result<u64> {
        let bytes = Self::read_bytes(buffer, offset, 8)?;
        Ok(u64::from_be_bytes(bytes.try_into().unwrap()))
    }

    fn read_u8(buffer: &[u8], offset: &mut usize) -> Result<u8> {
        let bytes = Self::read_bytes(buffer, offset, 1)?;
        Ok(bytes[0])
    }

    fn read_string(buffer: &[u8], offset: &mut usize, len: u32) -> Result<String> {
        let bytes = Self::read_bytes(buffer, offset, len as usize)?;
        std::str::from_utf8(bytes)
            .map(|s| s.to_string())
            .map_err(|e| Error::make_sys_error(Box::new(e), "BinParser::read_string"))
    }

    fn map_io_error(e: io::Error, context: &str) -> Error {
        if e.kind() == io::ErrorKind::UnexpectedEof {
            Error::ParseError(format!("Truncated record: {}", context))
        } else {
            Error::make_sys_error(Box::new(e), context)
        }
    }

    fn parse_record(&mut self) -> Result<Option<Transaction>> {
        let mut magic = [0u8; 4];
        match self.reader.read_exact(&mut magic) {
            Ok(_) => (),
            Err(e) => {
                if e.kind() == io::ErrorKind::UnexpectedEof {
                    return Ok(None);
                } else {
                    return Err(Error::make_sys_error(
                        Box::new(e),
                        "BinParser::parse_record::magic",
                    ));
                }
            }
        };

        if &magic != b"YPBN" {
            return Err(Error::ParseError(format!(
                "Invalid MAGIC: {:02X?} expected YPBN",
                magic
            )));
        };

        let mut buf = [0u8; 4];
        self.reader
            .read_exact(&mut buf)
            .map_err(|e| Self::map_io_error(e, "record size"))?;

        let record_size = u32::from_be_bytes(buf) as usize;
        let mut record_buffer = vec![0u8; record_size];
        self.reader
            .read_exact(&mut record_buffer)
            .map_err(|e| Self::map_io_error(e, "record body"))?;
        let mut offset = 0_usize;

        // TX_ID
        let tx_id = Self::read_u64(&record_buffer, &mut offset)?;

        // TX_TYPE
        let tx_type = Self::read_u8(&record_buffer, &mut offset)?;
        let operation = match tx_type {
            0 => TransactionType::Deposit,
            1 => TransactionType::Transfer,
            2 => TransactionType::Withdrawal,
            _ => return Err(Error::ParseError(format!("Invalid TX_TYPE: {}", tx_type))),
        };

        // FROM_USER_ID
        let from_user = Self::read_u64(&record_buffer, &mut offset)?;
        // TO_USER_ID
        let to_user = Self::read_u64(&record_buffer, &mut offset)?;
        // AMOUNT (unsigned; sign/direction is conveyed by TX_TYPE, see specs)
        let amount = Self::read_u64(&record_buffer, &mut offset)?;
        // TIMESTAMP
        let timestamp = Self::read_u64(&record_buffer, &mut offset)?;

        // STATUS
        let tx_status = Self::read_u8(&record_buffer, &mut offset)?;
        let status = match tx_status {
            0 => TransactionStatus::Success,
            1 => TransactionStatus::Failure,
            2 => TransactionStatus::Pending,
            _ => {
                return Err(Error::ParseError(format!(
                    "Invalid TX_STATUS: {}",
                    tx_status
                )));
            }
        };

        // DESCRIPTION
        let description_len = Self::read_u32(&record_buffer, &mut offset)?;
        let description = Self::read_string(&record_buffer, &mut offset, description_len)?;

        if offset != record_size {
            return Err(Error::ParseError(format!(
                "Record size mismatch: declared {}, consumed {}",
                record_size, offset
            )));
        }

        Ok(Some(Transaction::new(
            tx_id,
            operation,
            from_user,
            to_user,
            amount,
            timestamp,
            status,
            description,
        )))
    }
}

impl Iterator for BinIterator {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.stop_parse {
            return None;
        }

        match self.parse_record() {
            Ok(Some(tx)) => Some(Ok(tx)),
            Ok(None) => {
                self.stop_parse = true;
                None
            }
            Err(e) => {
                self.stop_parse = true;
                Some(Err(e))
            }
        }
    }
}

impl FormatDetector for BinParser {
    fn detect(buffer: &[u8]) -> bool {
        // See: YPBankBinFormat_ru.md
        buffer.len() >= 4 && &buffer[0..4] == b"YPBN"
    }
}

impl TransactionParser for BinParser {
    type Iter = BinIterator;

    fn parse(&self, reader: Box<dyn io::BufRead>) -> Result<Self::Iter> {
        Ok(BinIterator::new(reader))
    }
}

impl TransactionSerializer for BinSerializer {
    fn serialize(
        &self,
        writer: &mut dyn io::Write,
        transactions: &mut dyn Iterator<Item = Result<Transaction>>,
    ) -> Result<()> {
        for tx in transactions {
            let tx = tx.map_err(|e| e)?;
            let mut buffer = Vec::new();
            // TXID
            buffer.extend_from_slice(&tx.id.to_be_bytes());
            // TX_TYPE
            let operation = match tx.operation {
                TransactionType::Deposit => 0_u8,
                TransactionType::Transfer => 1_u8,
                TransactionType::Withdrawal => 2_u8,
            };
            buffer.extend_from_slice(&operation.to_be_bytes());
            // FROM_USER_ID
            buffer.extend_from_slice(&tx.from_user.to_be_bytes());
            // TO_USER_ID
            buffer.extend_from_slice(&tx.to_user.to_be_bytes());
            // AMOUNT
            buffer.extend_from_slice(&tx.amount.to_be_bytes());
            // TIMESTAMP
            buffer.extend_from_slice(&tx.timestamp.to_be_bytes());
            // STATUS
            let status = match tx.status {
                TransactionStatus::Success => 0u8,
                TransactionStatus::Failure => 1u8,
                TransactionStatus::Pending => 2u8,
            };
            buffer.extend_from_slice(&status.to_be_bytes());
            // DESCRIPTION length
            let desc_bytes = tx.description.as_bytes();
            let desc_len = desc_bytes.len() as u32;
            buffer.extend_from_slice(&desc_len.to_be_bytes());
            // DESCRIPTION
            buffer.extend_from_slice(desc_bytes);
            // MAGIC & Size
            writer
                .write_all(&[0x59, 0x50, 0x42, 0x4E])
                .map_err(|e| Error::make_sys_error(Box::new(e), "BinSerializer::write::magic"))?;
            writer
                .write_all(&(buffer.len() as u32).to_be_bytes())
                .map_err(|e| Error::make_sys_error(Box::new(e), "BinSerializer::write::size"))?;
            // Buffer
            writer
                .write_all(&buffer)
                .map_err(|e| Error::make_sys_error(Box::new(e), "BinSerializer::write::bbuffer"))?;
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
    fn test_parse_valid_binary() {
        let tx = Transaction::new(
            1234567890123456,
            TransactionType::Deposit,
            0,
            9876543210987654,
            10000,
            1633036800000,
            TransactionStatus::Success,
            "Terminal deposit".to_string(),
        );

        let mut data = Vec::new();
        data.extend_from_slice(b"YPBN");

        let mut body = Vec::new();
        body.extend_from_slice(&tx.id.to_be_bytes());
        body.extend_from_slice(&0u8.to_be_bytes());
        body.extend_from_slice(&tx.from_user.to_be_bytes());
        body.extend_from_slice(&tx.to_user.to_be_bytes());
        body.extend_from_slice(&tx.amount.to_be_bytes());
        body.extend_from_slice(&tx.timestamp.to_be_bytes());
        body.extend_from_slice(&0u8.to_be_bytes());

        let desc_bytes = tx.description.as_bytes();
        body.extend_from_slice(&(desc_bytes.len() as u32).to_be_bytes());
        body.extend_from_slice(desc_bytes);

        data.extend_from_slice(&(body.len() as u32).to_be_bytes());
        data.extend_from_slice(&body);

        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = BinParser;
        let mut iter = parser.parse(cursor).unwrap();

        let parsed = iter.next().unwrap().unwrap();
        assert_eq!(parsed.id, tx.id);
        assert_eq!(parsed.operation, tx.operation);
        assert_eq!(parsed.from_user, tx.from_user);
        assert_eq!(parsed.to_user, tx.to_user);
        assert_eq!(parsed.amount, tx.amount);
        assert_eq!(parsed.timestamp, tx.timestamp);
        assert_eq!(parsed.status, tx.status);
        assert_eq!(parsed.description, tx.description);

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_write_and_read_binary() {
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
            "Payment for services".to_string(),
        );

        let mut output = Vec::new();
        let transactions = vec![Ok(tx1.clone()), Ok(tx2.clone())];

        let mut iter = transactions.into_iter();
        let iter_ref: &mut dyn Iterator<Item = Result<Transaction>> = &mut iter;

        let serializer = BinSerializer;
        serializer.serialize(&mut output, iter_ref).unwrap();

        assert!(!output.is_empty());
        assert_eq!(&output[0..4], b"YPBN");

        let cursor = Box::new(BufReader::new(Cursor::new(output)));
        let parser = BinParser;
        let mut iter = parser.parse(cursor).unwrap();

        let parsed1 = iter.next().unwrap().unwrap();
        assert_eq!(parsed1.id, 1);
        assert_eq!(parsed1.operation, TransactionType::Deposit);
        assert_eq!(parsed1.from_user, tx1.from_user);
        assert_eq!(parsed1.to_user, tx1.to_user);
        assert_eq!(parsed1.amount, tx1.amount);
        assert_eq!(parsed1.timestamp, tx1.timestamp);
        assert_eq!(parsed1.status, tx1.status);
        assert_eq!(parsed1.description, tx1.description);

        let parsed2 = iter.next().unwrap().unwrap();
        assert_eq!(parsed2.id, 2);
        assert_eq!(parsed2.operation, TransactionType::Transfer);
        assert_eq!(parsed2.from_user, tx2.from_user);
        assert_eq!(parsed2.to_user, tx2.to_user);
        assert_eq!(parsed2.amount, tx2.amount);
        assert_eq!(parsed2.timestamp, tx2.timestamp);
        assert_eq!(parsed2.status, tx2.status);
        assert_eq!(parsed2.description, tx2.description);

        assert!(iter.next().is_none());
    }

    #[test]
    fn test_detect_bin_format() {
        assert!(BinParser::detect(b"YPBN"));
        assert!(BinParser::detect(b"YPBN some data"));
        assert!(!BinParser::detect(b"Some random text"));
        assert!(!BinParser::detect(b""));
        assert!(!BinParser::detect(b"YPB"));
    }

    fn parse_first(data: Vec<u8>) -> Result<Transaction> {
        let cursor = Box::new(BufReader::new(Cursor::new(data)));
        let parser = BinParser;
        let mut iter = parser.parse(cursor).unwrap();
        iter.next().unwrap()
    }

    #[test]
    fn test_truncated_record_body() {
        // MAGIC + RECORD_SIZE claiming 100 bytes, but almost no body follows
        let mut data = Vec::new();
        data.extend_from_slice(b"YPBN");
        data.extend_from_slice(&100u32.to_be_bytes());
        data.extend_from_slice(&[0u8; 10]);

        let err = parse_first(data).unwrap_err();
        assert!(
            err.to_string().contains("Truncated record"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_record_size_too_small() {
        // RECORD_SIZE smaller than fixed fields require (~45 bytes without description)
        let mut data = Vec::new();
        data.extend_from_slice(b"YPBN");
        data.extend_from_slice(&8u32.to_be_bytes());
        data.extend_from_slice(&[0u8; 8]);

        let err = parse_first(data).unwrap_err();
        assert!(
            err.to_string().contains("Truncated record"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_record_size_too_large_trailing_bytes() {
        let tx = Transaction::new(
            1,
            TransactionType::Deposit,
            0,
            1001,
            50000,
            1672531200000,
            TransactionStatus::Success,
            "ok".to_string(),
        );

        let mut body = Vec::new();
        body.extend_from_slice(&tx.id.to_be_bytes());
        body.extend_from_slice(&0u8.to_be_bytes());
        body.extend_from_slice(&tx.from_user.to_be_bytes());
        body.extend_from_slice(&tx.to_user.to_be_bytes());
        body.extend_from_slice(&tx.amount.to_be_bytes());
        body.extend_from_slice(&tx.timestamp.to_be_bytes());
        body.extend_from_slice(&0u8.to_be_bytes());
        let desc_bytes = tx.description.as_bytes();
        body.extend_from_slice(&(desc_bytes.len() as u32).to_be_bytes());
        body.extend_from_slice(desc_bytes);
        // Extra trailing bytes inside declared record size
        body.extend_from_slice(&[0xAA, 0xBB]);

        let mut data = Vec::new();
        data.extend_from_slice(b"YPBN");
        data.extend_from_slice(&(body.len() as u32).to_be_bytes());
        data.extend_from_slice(&body);

        let err = parse_first(data).unwrap_err();
        assert!(
            err.to_string().contains("Record size mismatch"),
            "unexpected error: {}",
            err
        );
    }

    #[test]
    fn test_invalid_magic_in_stream() {
        let mut data = Vec::new();
        data.extend_from_slice(b"XXXX");
        data.extend_from_slice(&0u32.to_be_bytes());

        let err = parse_first(data).unwrap_err();
        assert!(
            err.to_string().contains("Invalid MAGIC"),
            "unexpected error: {}",
            err
        );
    }
}
