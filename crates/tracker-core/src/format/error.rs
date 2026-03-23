use thiserror::Error;

pub type FormatResult<T> = Result<T, FormatError>;

#[derive(Debug, Error, Clone, PartialEq)]
pub enum FormatError {
    #[error("File too short: {0}")]
    TruncatedFile(String),

    #[error("Invalid header: {0}")]
    InvalidHeader(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Parse error at offset {offset}: {message}")]
    ParseError { offset: usize, message: String },

    #[error("Invalid pattern data: {0}")]
    InvalidPattern(String),

    #[error("Invalid instrument data: {0}")]
    InvalidInstrument(String),

    #[error("Invalid sample data: {0}")]
    InvalidSample(String),

    #[error("Missing required field: {0}")]
    MissingField(String),

    #[error("Corrupted data: {0}")]
    Corruption(String),

    #[error("{0}")]
    Other(String),
}

impl From<String> for FormatError {
    fn from(s: String) -> Self {
        FormatError::Other(s)
    }
}

impl From<&str> for FormatError {
    fn from(s: &str) -> Self {
        FormatError::Other(s.to_string())
    }
}

impl FormatError {
    pub fn truncated_file(expected: &str) -> Self {
        FormatError::TruncatedFile(expected.to_string())
    }

    pub fn invalid_header(msg: &str) -> Self {
        FormatError::InvalidHeader(msg.to_string())
    }

    pub fn unsupported_format(msg: &str) -> Self {
        FormatError::UnsupportedFormat(msg.to_string())
    }

    pub fn parse_error(offset: usize, message: &str) -> Self {
        FormatError::ParseError {
            offset,
            message: message.to_string(),
        }
    }

    pub fn invalid_pattern(msg: &str) -> Self {
        FormatError::InvalidPattern(msg.to_string())
    }

    pub fn invalid_instrument(msg: &str) -> Self {
        FormatError::InvalidInstrument(msg.to_string())
    }

    pub fn invalid_sample(msg: &str) -> Self {
        FormatError::InvalidSample(msg.to_string())
    }

    pub fn missing_field(field: &str) -> Self {
        FormatError::MissingField(field.to_string())
    }

    pub fn corruption(msg: &str) -> Self {
        FormatError::Corruption(msg.to_string())
    }
}

pub trait FormatValidator {
    fn validate(&self) -> FormatResult<()>;
}

impl FormatValidator for [u8] {
    fn validate(&self) -> FormatResult<()> {
        if self.is_empty() {
            return Err(FormatError::truncated_file("empty file"));
        }
        Ok(())
    }
}
