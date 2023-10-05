
#[derive(thiserror::Error, Debug)]
pub enum PtError {
    #[error("input file does not exist: {0}")]
    FileNotFound(String),
    #[error("failed to read file: {0}")]
    FileReadError(#[from] std::io::Error),
    #[error("proto parsing failed: {0}")]
    ParsingError(String),
    #[error("proto parsing was incomplete")]
    IncompleteParsing,
    #[error("could not find type named: {0}")]
    ProtobufTypeNotFound(String),
}
