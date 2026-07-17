use std::env;
use std::fs;
use std::io;
use std::process::ExitCode;

use parser::{Error, Format, Result, Transaction, get_parser};

fn print_usage() {
    println!("Transaction Comparer");
    println!();
    println!("Usage:");
    println!("  comparer <FILE1> <FILE2>");
    println!();
    println!("Options:");
    println!("  --help    Show this help");
    println!();
    println!("Examples:");
    println!("  comparer data1.csv data2.txt");
    println!("  comparer data1.bin data2.csv");
}

struct CliArgs {
    files: Vec<String>,
    help: bool,
}

fn parse_cmd_line() -> CliArgs {
    let raw_args: Vec<String> = env::args().skip(1).collect();
    let mut res = CliArgs {
        files: Vec::new(),
        help: false,
    };

    for arg in raw_args {
        if arg != "--help" {
            res.files.push(arg.to_string());
        } else {
            res.help = true;
            return res;
        }
    }

    res
}

fn compare_sequences<I1, I2>(
    mut iter1: I1,
    mut iter2: I2,
    file1_path: &str,
    file2_path: &str,
) -> Result<()>
where
    I1: Iterator<Item = Result<Transaction>>,
    I2: Iterator<Item = Result<Transaction>>,
{
    loop {
        let tx1 = iter1.next();
        let tx2 = iter2.next();

        match (tx1, tx2) {
            (None, None) => {
                println!("All transactions match!");
                return Ok(());
            }
            (None, Some(Ok(_))) => {
                return Err(Error::ParseError(format!(
                    "{} has extra transactions",
                    file2_path,
                )));
            }
            (Some(Ok(_)), None) => {
                return Err(Error::ParseError(format!(
                    "{} has extra transaction at line",
                    file1_path,
                )));
            }
            (Some(Err(e)), Some(_) | None) => {
                return Err(Error::ParseError(format!(
                    "Error reading from {}: {}",
                    file1_path, e
                )));
            }
            (Some(_) | None, Some(Err(e))) => {
                return Err(Error::ParseError(format!(
                    "Error reading from {}: {}",
                    file2_path, e
                )));
            }
            (Some(Ok(tx1)), Some(Ok(tx2))) => {
                if let Some(diff_description) = tx1.diff(&tx2) {
                    return Err(Error::ParseError(format!(
                        "Transactions arent match! The difference is: {}",
                        diff_description,
                    )));
                }
            }
        }
    }
}

fn run() -> Result<()> {
    let args = parse_cmd_line();

    if args.help {
        print_usage();
        return Ok(());
    }

    if args.files.len() != 2 {
        return Err(Error::MissingArgument(
            "Exactly two files are required".to_string(),
        ));
    };

    let path_f1 = args.files[0].as_str();
    let mut file_1: Box<dyn io::BufRead> = {
        let f = fs::File::open(path_f1).map_err(|e| Error::make_sys_error(Box::new(e), path_f1))?;
        Box::new(io::BufReader::new(f))
    };

    let path_f2 = args.files[1].as_str();
    let mut file_2: Box<dyn io::BufRead> = {
        let f = fs::File::open(path_f2).map_err(|e| Error::make_sys_error(Box::new(e), path_f2))?;
        Box::new(io::BufReader::new(f))
    };

    let file1_format = Format::detect_from_content(&mut file_1)?;
    let file2_format = Format::detect_from_content(&mut file_2)?;

    let f1_parser = get_parser(file1_format);
    let f2_parser = get_parser(file2_format);

    let iter_1 = f1_parser.parse(file_1)?;
    let iter_2 = f2_parser.parse(file_2)?;

    compare_sequences(iter_1, iter_2, path_f1, path_f2)?;
    Ok(())
}

fn main() -> ExitCode {
    let result = run();
    match result {
        Ok(_) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", e);
            ExitCode::FAILURE
        }
    }
}
