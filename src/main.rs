use self::errors::PtError;

mod errors;
mod parser;

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

fn process() -> Result<(), PtError> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() < 2 {
        usage(&args[0]);
        std::process::exit(1);
    };

    let input_file = &args[1];
    let input = read(input_file)?;
    let _parsed = parser::parse(input)?;

    Ok(())
}

fn main() {
    match process() {
        Ok(()) => {}
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(2);
        }
    }
}
