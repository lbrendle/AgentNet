use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid cbor: {0}")]
    Cbor(&'static str),
    #[error("io error: {0}")]
    Io(String),
    #[error("invalid utf-8")]
    Utf8,
    #[error("unsupported cbor type")]
    Unsupported,
    #[error("non-canonical length encoding")]
    NonCanonicalLength,
    #[error("indefinite length is not allowed")]
    IndefiniteLength,
    #[error("integer overflow")]
    IntegerOverflow,
    #[error("trailing bytes after cbor value")]
    TrailingBytes,
    #[error("duplicate map key")]
    DuplicateKey,
    #[error("invalid signature")]
    InvalidSignature,
}
