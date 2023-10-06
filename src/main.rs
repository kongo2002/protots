use self::errors::PtError;

mod errors;
mod parser;
mod typescript;

pub struct Opts {
    file: String,
    verbose: bool,
}

fn read(input_file: &str) -> Result<String, PtError> {
    if !std::path::Path::new(input_file).exists() {
        return Err(PtError::FileNotFound(input_file.to_owned()));
    }

    let content = std::fs::read_to_string(input_file)?;
    Ok(content)
}

fn usage(program: &str) {
    println!("{} <FILE> [OPTIONS]", program);
}

fn opts(mut args: Vec<String>) -> Opts {
    let mut has_arg = |opt: &str| {
        if let Some(idx) = args.iter().position(|val| val == opt) {
            args.remove(idx);
            true
        } else {
            false
        }
    };

    let verbose = has_arg("-v");

    if args.len() < 2 {
        usage(&args[0]);
        std::process::exit(2);
    }

    Opts {
        file: args.remove(1),
        verbose,
    }
}

fn process() -> Result<(), PtError> {
    let opts = opts(std::env::args().collect());

    let input = read(&opts.file)?;
    let proto = parser::parse(&opts, &input)?;
    let ts_schema = typescript::to_schema(&proto)?;

    println!("{}", ts_schema);

    Ok(())
}

fn main() {
    match process() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}
