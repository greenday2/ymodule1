use std::io;

use super::transaction::{Transaction, TransactionStatus, TransactionType};
use crate::error::{Error, Result};
use crate::parsers::{FormatDetector, TransactionReader, TransactionWriter};

pub struct BinParser;

pub struct BinIterator {
    fake_txs: Vec<Transaction>,
    fake_idx: usize,
}

impl BinIterator {
    pub fn new() -> Self {
        BinIterator {
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
                    description: "BIN Payment for services 1".to_string(),
                },
                Transaction {
                    id: 3,
                    operation: TransactionType::Withdrawal,
                    from_user: 1001,
                    to_user: 1002,
                    amount: 15000,
                    timestamp: 1672534800000,
                    status: TransactionStatus::Pending,
                    description: "BIN Payment for services 2".to_string(),
                },
                Transaction {
                    id: 4,
                    operation: TransactionType::Deposit,
                    from_user: 1001,
                    to_user: 1002,
                    amount: 15000,
                    timestamp: 1672534800000,
                    status: TransactionStatus::Success,
                    description: "BIN Payment for services 3".to_string(),
                },
            ],
        }
    }
}

impl Iterator for BinIterator {
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

impl FormatDetector for BinParser {
    fn detect(buffer: &[u8]) -> bool {
        // See: YPBankBinFormat_ru.md
        buffer.len() >= 4 && &buffer[0..4] == b"YPBN"
    }
}

impl TransactionReader for BinParser {
    type Iter = BinIterator;

    fn read_transactions<R: io::Read + 'static>(&self, reader: R) -> Result<Self::Iter> {
        Ok(BinIterator::new())
    }
}

impl TransactionWriter for BinParser {
    fn write_transactions<W: io::Write, I: Iterator<Item = Result<Transaction>>>(
        &self,
        mut writer: W,
        transactions: I,
    ) -> Result<()> {
        writeln!(
            writer,
            "BIN\nTX_ID,TX_TYPE,FROM_USER_ID,TO_USER_ID,AMOUNT,TIMESTAMP,STATUS,DESCRIPTION"
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
