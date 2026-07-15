mod bin_parser;
mod csv_parser;
mod transaction;
mod txt_parser;

pub use bin_parser::BinParser;
pub use csv_parser::CsvParser;
pub use txt_parser::TxtParser;

use crate::error::{Error, Result};
use std::io;
use transaction::Transaction;

pub trait TransactionReader {
    type Iter: Iterator<Item = Result<Transaction>>;

    fn read_transactions<R: io::Read + 'static>(&self, reader: R) -> Result<Self::Iter>;
}

pub trait TransactionWriter {
    fn write_transactions<W: io::Write, I: Iterator<Item = Result<Transaction>>>(
        &self,
        writer: W,
        transactions: I,
    ) -> Result<()>;
}

pub trait FormatDetector {
    fn detect(buffer: &[u8]) -> bool;
}

#[derive(PartialEq)]
pub enum Format {
    Csv,
    Txt,
    Bin,
}

impl Format {
    pub fn detect_from_content<R: io::BufRead>(reader: &mut R) -> Result<Self> {
        let buffer = reader
            .fill_buf()
            .map_err(|e| Error::make_io_error(e, "Failed to read for format detection"))?;

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
            "csv" => Ok(Format::Csv),
            "txt" => Ok(Format::Txt),
            "bin" => Ok(Format::Bin),
            _ => Err(Error::InvalidFormat(format!(
                "Unknown format: {}. Supported txt, bin, csv.",
                format_name
            ))),
        }
    }

    pub fn the_same_formats(in_fmt: &Format, out_fmt: &Format) -> Result<()> {
        if in_fmt != out_fmt {
            Ok(())
        } else {
            Err(Error::InvalidFormat(format!(
                "Both Input and Output formats are the Same. \
                \nTell me dr.Freeman, Why Should I engage in useless work?"
            )))
        }
    }
}

pub enum Reader {
    Csv(CsvParser),
    Txt(TxtParser),
    Bin(BinParser),
}

impl Reader {
    pub fn read_transactions<R: io::Read + 'static>(
        &self,
        reader: R,
    ) -> Result<Box<dyn Iterator<Item = Result<Transaction>>>> {
        match self {
            Reader::Csv(parser) => {
                let iter = parser.read_transactions(reader)?;
                Ok(Box::new(iter))
            }
            Reader::Txt(parser) => {
                let iter = parser.read_transactions(reader)?;
                Ok(Box::new(iter))
            }
            Reader::Bin(parser) => {
                let iter = parser.read_transactions(reader)?;
                Ok(Box::new(iter))
            }
        }
    }
}

pub enum Writer {
    Csv(CsvParser),
    Txt(TxtParser),
    Bin(BinParser),
}

impl Writer {
    pub fn write_transactions<W: io::Write, I: Iterator<Item = Result<Transaction>>>(
        &self,
        writer: W,
        transactions: I,
    ) -> Result<()> {
        match self {
            Writer::Csv(parser) => parser.write_transactions(writer, transactions),
            Writer::Txt(parser) => parser.write_transactions(writer, transactions),
            Writer::Bin(parser) => parser.write_transactions(writer, transactions),
        }
    }
}

pub fn get_reader(format: Format) -> Reader {
    match format {
        Format::Csv => Reader::Csv(CsvParser),
        Format::Txt => Reader::Txt(TxtParser),
        Format::Bin => Reader::Bin(BinParser),
    }
}

pub fn get_writer(format: Format) -> Writer {
    match format {
        Format::Csv => Writer::Csv(CsvParser),
        Format::Txt => Writer::Txt(TxtParser),
        Format::Bin => Writer::Bin(BinParser),
    }
}
