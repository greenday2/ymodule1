use std::io;

mod bin_parser;
mod csv_parser;
mod transaction;
mod txt_parser;

use bin_parser::BinParser;
use bin_parser::BinSerializer;

use csv_parser::CsvParser;
use csv_parser::CsvSerializer;

use txt_parser::TxtParser;
use txt_parser::TxtSerializer;

pub use transaction::Transaction;

use crate::{Error, Result};

pub trait TransactionParser {
    type Iter: Iterator<Item = Result<Transaction>>;

    fn parse(&self, reader: Box<dyn io::BufRead>) -> Result<Self::Iter>;
}

pub trait TransactionSerializer {
    fn serialize(
        &self,
        writer: &mut dyn io::Write,
        transactions: &mut dyn Iterator<Item = Result<Transaction>>,
    ) -> Result<()>;
}

pub trait FormatDetector {
    fn detect(buffer: &[u8]) -> bool;
}

#[derive(Debug, Clone, Copy)]
pub enum Format {
    Csv,
    Txt,
    Bin,
}

impl Format {
    pub fn detect_from_content(reader: &mut dyn io::BufRead) -> Result<Self> {
        let buffer = reader.fill_buf().map_err(|e| {
            Error::make_sys_error(Box::new(e), "Failed to read for format detection")
        })?;

        if BinParser::detect(buffer) {
            Ok(Format::Bin)
        } else if CsvParser::detect(buffer) {
            Ok(Format::Csv)
        } else if TxtParser::detect(buffer) {
            Ok(Format::Txt)
        } else {
            Err(Error::InvalidFormat(
                "Cant detect input format!".to_string(),
            ))
        }
    }

    pub fn from_str(format_name: &str) -> Result<Self> {
        match format_name {
            "bin" => Ok(Format::Bin),
            "csv" => Ok(Format::Csv),
            "txt" => Ok(Format::Txt),
            _ => Err(Error::InvalidFormat(format!(
                "Unknown format: {}. Supported txt, bin, csv.",
                format_name
            ))),
        }
    }
}

pub enum Parser {
    Bin(BinParser),
    Csv(CsvParser),
    Txt(TxtParser),
}

impl Parser {
    pub fn parse(
        &self,
        reader: Box<dyn io::BufRead>,
    ) -> Result<Box<dyn Iterator<Item = Result<Transaction>>>> {
        match self {
            Parser::Bin(parser) => Ok(Box::new(parser.parse(reader)?)),
            Parser::Csv(parser) => Ok(Box::new(parser.parse(reader)?)),
            Parser::Txt(parser) => Ok(Box::new(parser.parse(reader)?)),
        }
    }
}

pub fn get_parser(fmt: Format) -> Parser {
    match fmt {
        Format::Bin => Parser::Bin(BinParser),
        Format::Csv => Parser::Csv(CsvParser),
        Format::Txt => Parser::Txt(TxtParser),
    }
}

pub enum Serializer {
    Bin(BinSerializer),
    Csv(CsvSerializer),
    Txt(TxtSerializer),
}

impl Serializer {
    pub fn serialize(
        &self,
        writer: &mut dyn io::Write,
        transactions: &mut dyn Iterator<Item = Result<Transaction>>,
    ) -> Result<()> {
        match self {
            Serializer::Bin(serializer) => serializer.serialize(writer, transactions),
            Serializer::Csv(serializer) => serializer.serialize(writer, transactions),
            Serializer::Txt(serializer) => serializer.serialize(writer, transactions),
        }
    }
}

pub fn get_serializer(fmt: Format) -> Serializer {
    match fmt {
        Format::Bin => Serializer::Bin(BinSerializer),
        Format::Csv => Serializer::Csv(CsvSerializer),
        Format::Txt => Serializer::Txt(TxtSerializer),
    }
}

#[cfg(test)]
mod description_roundtrip_tests {
    use super::*;
    use crate::Result;
    use std::io::{BufReader, Cursor};

    use transaction::{Transaction, TransactionStatus, TransactionType};

    fn sample_tx() -> Transaction {
        Transaction::new(
            42,
            TransactionType::Transfer,
            1,
            2,
            1000,
            1234567890,
            TransactionStatus::Success,
            r#"Payment: "invoice" #1"#.to_string(),
        )
    }

    fn roundtrip(from: Format, to: Format, tx: &Transaction) -> Result<Transaction> {
        let mut encoded = Vec::new();
        {
            let mut iter = vec![Ok(tx.clone())].into_iter();
            let iter_ref: &mut dyn Iterator<Item = Result<Transaction>> = &mut iter;
            get_serializer(from).serialize(&mut encoded, iter_ref)?;
        }

        // Re-encode through intermediate format `to`, then parse back as `to`
        let mut converted = Vec::new();
        {
            let reader = Box::new(BufReader::new(Cursor::new(encoded)));
            let mut parsed = get_parser(from).parse(reader)?;
            get_serializer(to).serialize(&mut converted, &mut parsed)?;
        }

        let reader = Box::new(BufReader::new(Cursor::new(converted)));
        let mut iter = get_parser(to).parse(reader)?;
        iter.next().unwrap()
    }

    #[test]
    fn description_survives_txt_csv_bin_conversions() {
        let tx = sample_tx();

        for (from, to) in [
            (Format::Txt, Format::Csv),
            (Format::Csv, Format::Bin),
            (Format::Bin, Format::Txt),
            (Format::Txt, Format::Bin),
            (Format::Bin, Format::Csv),
            (Format::Csv, Format::Txt),
        ] {
            let parsed = roundtrip(from, to, &tx).unwrap();
            assert!(
                parsed.diff(&tx).is_none(),
                "roundtrip {:?} -> {:?} failed: {:?}",
                from,
                to,
                parsed.diff(&tx)
            );
        }
    }
}
