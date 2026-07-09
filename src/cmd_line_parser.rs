use std::collections::HashMap;
use std::env;

struct CmdLineArgs {
    back: Vec<String>,
    original: std::env::Args,
}

impl CmdLineArgs {
    pub fn new() -> Self {
        CmdLineArgs {
            original: env::args(),
            back: Vec::with_capacity(2),
        }
    }

    pub fn push_back(&mut self, arg: String) {
        self.back.push(arg);
    }

    fn next(&mut self) -> Option<String> {
        if let Some(arg) = self.back.pop() {
            Some(arg)
        } else {
            self.original.next()
        }
    }
}

enum CmdLineArgValueKind {
    Required,
    NotRequired,
}

pub enum CmdLineParserResult {
    NoArgs,
    HelpSpecified,
    ParseError(String),
    Args(HashMap<String, Option<String>>),
}

pub fn parse_command_line_args() -> CmdLineParserResult {
    let possible_args = HashMap::<&str, CmdLineArgValueKind>::from([
        ("--in", CmdLineArgValueKind::Required),
        ("--out", CmdLineArgValueKind::Required),
        ("--dest-format", CmdLineArgValueKind::Required),
        ("--help", CmdLineArgValueKind::NotRequired),
    ]);

    let mut collected_args: HashMap<String, Option<String>> =
        HashMap::with_capacity(possible_args.len());

    enum CmdLineParserState {
        ReadArg,
        ReadVal(String),
    }

    let mut args = CmdLineArgs::new();
    args.next(); // skip program name

    let mut result = CmdLineParserResult::NoArgs;
    let mut parser_state = CmdLineParserState::ReadArg;
    while let Some(arg) = args.next() {
        match parser_state {
            CmdLineParserState::ReadArg => {
                if let Some((arg, val)) = arg.split_once('=') {
                    args.push_back(val.to_string());
                    args.push_back(arg.to_string());
                } else {
                    let value_kind = possible_args.get(&arg.as_str());
                    if let Some(value_kind) = value_kind {
                        match *value_kind {
                            CmdLineArgValueKind::NotRequired => {
                                collected_args.insert(arg, None);
                            }
                            CmdLineArgValueKind::Required => {
                                parser_state = CmdLineParserState::ReadVal(arg)
                            }
                        }
                    } else {
                        let err_msg = format!("Invalid argument: {}", arg);
                        result = CmdLineParserResult::ParseError(err_msg);
                        break;
                    }
                }
            }
            CmdLineParserState::ReadVal(ref arg_name) => {
                if possible_args.contains_key(arg.as_str()) {
                    break;
                }

                collected_args.insert(arg_name.clone(), Some(arg));
                parser_state = CmdLineParserState::ReadArg;
            }
        }
    }

    if let CmdLineParserState::ReadVal(arg_name) = parser_state {
        let err_msg = format!("Argument {} needs value", arg_name);
        result = CmdLineParserResult::ParseError(err_msg)
    }

    if let CmdLineParserResult::NoArgs = result {
        if !collected_args.is_empty() {
            if collected_args.contains_key("--help") {
                result = CmdLineParserResult::HelpSpecified;
            } else if !collected_args.contains_key("--dest-format") {
                result = CmdLineParserResult::ParseError(format!("--dest-format required"))
            } else {
                result = CmdLineParserResult::Args(collected_args);
            }
        }
    }

    result
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
