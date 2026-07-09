// use std::time::{SystemTime, UNIX_EPOCH};
mod cmd_line_parser;
use cmd_line_parser::{CmdLineParserResult, parse_command_line_args, print_help};

fn main() {
    let cmd_parse_result = parse_command_line_args();
    match cmd_parse_result {
        CmdLineParserResult::NoArgs | CmdLineParserResult::HelpSpecified => print_help(),
        CmdLineParserResult::ParseError(err_msg) => {
            eprintln!("{}\nUse converter --help for more information.", err_msg);
        }
        CmdLineParserResult::Args(_) => (),
    }
}
