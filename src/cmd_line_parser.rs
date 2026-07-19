use std::env;

use parser::{Error, Result};

#[derive(Debug, Default, Clone)]
pub struct CliArgs {
    pub output_format: String,
    pub input_file: Option<String>,
    pub output_file: Option<String>,
    pub help: bool,
}

impl CliArgs {
    pub fn parse() -> Result<Self> {
        let mut cli_args = CliArgs::default();
        let mut args = env::args().skip(1).peekable(); // skip program name
        let arg_requires_value = |arg| format!("Argument {} requires a value", arg);

        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--in" | "--out" | "--dest-format" => {
                    if let Some(v) = args.next() {
                        if !v.starts_with("--") {
                            if arg == "--in" {
                                cli_args.input_file = Some(v)
                            } else if arg == "--out" {
                                cli_args.output_file = Some(v)
                            } else if arg == "--dest-format" {
                                cli_args.output_format = v
                            }
                        } else {
                            return Err(Error::MissingArgument(arg_requires_value(arg)));
                        }
                    } else {
                        return Err(Error::MissingArgument(arg_requires_value(arg)));
                    };
                }
                "--help" => {
                    cli_args.help = true;
                }
                _ => {
                    return Err(Error::UnknownArgument(format!("Unknown argument: {}", arg)));
                }
            }
        }

        if !cli_args.help && cli_args.output_format.is_empty() {
            return Err(Error::MissingArgument(
                "--dest-format is required".to_string(),
            ));
        }

        Ok(cli_args)
    }
}

pub fn print_help() {
    println!(
        r#"
Usage: converter OPTIONS

    Options:
      --help   Show this help message
      --in     Path to input file (STDIN if not specified)
      --out    Path to output file (STDOUT if not specified)

      --dest-format (required)
               Output format. Supported: txt, bin, csv

    Examples:
      converter --in file1.csv --dest-format txt
      cat file.txt | converter --out file1.bin --dest-format bin
    "#
    );
}
