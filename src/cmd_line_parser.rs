use std::collections::{HashMap, HashSet};
use std::env;

pub type ArgsMap = HashMap<String, Option<String>>;

pub fn parse_command_line_args() -> Result<ArgsMap, String> {
    let possible_args = HashSet::<&str>::from_iter(["--help", "--in", "--out", "--dest-format"]);
    let mut collected_args: ArgsMap = HashMap::new();

    let mut args = env::args().skip(1).peekable(); // skip program name
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--in" | "--out" | "--dest-format" => {
                let arg_needs_value = || format!("Argument {} needs value", arg);
                if let Some(v) = args.next() {
                    if !possible_args.contains(v.as_str()) {
                        collected_args.insert(arg, Some(v));
                    } else {
                        return Err(arg_needs_value());
                    }
                } else {
                    return Err(arg_needs_value());
                };
            }
            "--help" => {
                collected_args.insert(arg, None);
            }
            _ => {
                return Err(format!("Unknow argument: {}", arg));
            }
        }
    }

    Ok(collected_args)
}

pub fn print_help() {
    println!(
        r#"
Usage: converer OPTIONS

    Options:
      --help   Show this help message
      --in     Path to input file (STDIN if not specified)
      --out    Path to output file (STDOUT if not specified)

      --dest-format (required)
               Output format. Supported: txt, bin, csv

    Examples:
      converter --in file1.csv --dest-format txt
      cat file.txt | converter --out file1.csv --dest-format bin
    "#
    );
}
