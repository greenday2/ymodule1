// use std::time::{SystemTime, UNIX_EPOCH};

mod cmd_line_parser;
use cmd_line_parser::{parse_command_line_args, print_help};

fn main() {
    const USE_MESSAGE: &str = "Use converter --help for more details.";
    let result = parse_command_line_args();
    match result {
        Err(msg) => eprintln!("{}\n{}", msg, USE_MESSAGE),
        Ok(args) => {
            if args.is_empty() || args.contains_key("--help") {
                print_help();
            } else if !args.contains_key("--dest-format") {
                eprintln!("--dest-format is required!\n{}", USE_MESSAGE)
            } else {
                ()
            }
        }
    }
}
