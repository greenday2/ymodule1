use std::io;

use super::transaction::{Transaction, TransactionStatus, TransactionType};
use crate::error::{Error, Result};
use crate::parsers::{FormatDetector, TransactionReader, TransactionWriter};

pub struct TxtParser;

pub struct TxtIterator {
    fake_txs: Vec<Transaction>,
    fake_idx: usize,
}

impl TxtIterator {
    pub fn new() -> Self {
        TxtIterator {
            fake_idx: 0,
            fake_txs: vec![
                Transaction {
                    id: 2,
                    operation: TransactionType::Transfer,
                    from_user: 1001,
                    to_user: 1002,
                    amount: 15000,
                    timestamp: 1672534800000,
                    status: TransactionStatus::Failure,
                    description: "TXT Payment for services 1".to_string(),
                },
                Transaction {
                    id: 3,
                    operation: TransactionType::Withdrawal,
                    from_user: 1001,
                    to_user: 1002,
                    amount: 15000,
                    timestamp: 1672534800000,
                    status: TransactionStatus::Pending,
                    description: "TXT Payment for services 2".to_string(),
                },
                Transaction {
                    id: 4,
                    operation: TransactionType::Deposit,
                    from_user: 1001,
                    to_user: 1002,
                    amount: 15000,
                    timestamp: 1672534800000,
                    status: TransactionStatus::Success,
                    description: "TXT Payment for services 3".to_string(),
                },
            ],
        }
    }
}

impl Iterator for TxtIterator {
    type Item = Result<Transaction>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.fake_idx < self.fake_txs.len() {
            let tx = self.fake_txs[self.fake_idx].clone();
            self.fake_idx += 1;
            Some(Ok(tx))
        } else {
            None
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

impl TransactionReader for TxtParser {
    type Iter = TxtIterator;

    fn read_transactions<R: io::Read + 'static>(&self, reader: R) -> Result<Self::Iter> {
        Ok(TxtIterator::new())
    }
}

impl TransactionWriter for TxtParser {
    fn write_transactions<W: io::Write, I: Iterator<Item = Result<Transaction>>>(
        &self,
        mut writer: W,
        transactions: I,
    ) -> Result<()> {
        writeln!(
            writer,
            "TXT\nTX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION"
        )
        .map_err(|e| Error::make_io_error(e, "While writing data to the output"))?;

        for res in transactions {
            let tx = res?;
            writeln!(
                writer,
                "{},{},{},{},{},{},{},\"{}\"",
                tx.id,
                tx.operation,
                tx.from_user,
                tx.to_user,
                tx.amount,
                tx.timestamp,
                tx.status,
                tx.description
            )
            .map_err(|e| Error::make_io_error(e, "While writing data to the output"))?;
        }

        Ok(())
    }
}
