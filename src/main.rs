mod cmd_line_parser;

use std::{
    fs::File,
    io::{self, BufReader, Write},
    process::ExitCode,
};

use cmd_line_parser::{CliArgs, print_help};
use parser::{
    error::{Error, Result},
    parsers::{Format, get_reader, get_writer},
};

fn run() -> Result<()> {
    let cmd_args = CliArgs::parse()?;

    if cmd_args.help {
        print_help();
        return Ok(());
    }

    let output_format = Format::from_str(&cmd_args.output_format.as_str())?;

    let mut input_data: Box<dyn io::BufRead> = if let Some(path) = &cmd_args.input_file {
        let f = File::open(path).map_err(|e| Error::make_io_error(e, path))?;
        Box::new(BufReader::new(f))
    } else {
        Box::new(BufReader::new(io::stdin()))
    };

    let input_format = Format::detect_from_content(&mut input_data)?;

    let mut output_data: Box<dyn Write> = if let Some(path) = &cmd_args.output_file {
        Box::new(File::create(path).map_err(|e| Error::make_io_error(e, path))?)
    } else {
        Box::new(io::stdout())
    };

    Format::the_same_formats(&input_format, &output_format)?;

    let reader = get_reader(input_format);
    let writer = get_writer(output_format);

    let tx_iter = reader.read_transactions(input_data)?;
    writer.write_transactions(&mut output_data, tx_iter)?;
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
