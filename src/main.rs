mod cmd_line_parser;

use std::{
    fs::File,
    io::{self, BufReader, Write},
    process::ExitCode,
};

use cmd_line_parser::{CliArgs, print_help};
use parser::{Error, Result};
use parser::{Format, get_parser, get_serializer};

fn run() -> Result<()> {
    let cmd_args = CliArgs::parse()?;

    if cmd_args.help {
        print_help();
        return Ok(());
    }

    let output_format = Format::from_str(&cmd_args.output_format.as_str())?;

    let mut input_data: Box<dyn io::BufRead> = if let Some(path) = &cmd_args.input_file {
        let f = File::open(path).map_err(|e| Error::make_sys_error(Box::new(e), path))?;
        Box::new(BufReader::new(f))
    } else {
        Box::new(io::stdin().lock())
    };

    let input_format = Format::detect_from_content(&mut input_data)?;

    let output_data: &mut dyn Write = if let Some(path) = &cmd_args.output_file {
        &mut File::create(path).map_err(|e| Error::make_sys_error(Box::new(e), path))?
    } else {
        &mut io::stdout().lock()
    };

    let parser = get_parser(input_format);
    let serializer = get_serializer(output_format);
    let mut tx_iter = parser.parse(input_data)?;
    serializer.serialize(output_data, &mut tx_iter)?;

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
