//! Strongly typed error definitions for request body parsing and quota enforcement.

use std::{error::Error, fmt};

use router::{RouterError, StatusCode};

/// Error conditions encountered during request body ingestion and parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BodyParserError {
    /// Request payload exceeded the configured byte ceiling (RFC 9110 §15.5.14).
    PayloadTooLarge {
        /// The maximum allowed byte limit.
        limit: usize,
        /// The number of bytes received when limit was crossed.
        received: usize,
    },
    /// The multipart stream or boundary delimiter was malformed (RFC 7578).
    MalformedMultipart(String),
    /// The request Content-Type is invalid or missing required parameters (e.g. boundary).
    InvalidContentType(String),
    /// The specified character encoding is unsupported.
    UnsupportedEncoding(String),
    /// Container nesting depth in JSON or form inputs exceeded the configured limit.
    NestingDepthExceeded {
        /// Maximum allowed container depth.
        max_depth: usize,
    },
    /// The number of form fields exceeded the configured limit.
    FieldCountExceeded {
        /// Maximum allowed field count.
        limit: usize,
    },
    /// An I/O error occurred during streaming or temporary file spooling.
    IoError(String),
    /// JSON syntax or deserialization failed.
    JsonError(String),
    /// URL-encoded form parsing failed.
    FormError(String),
    /// Payload contained invalid UTF-8 byte sequences.
    Utf8Error(String),
    /// The uploaded file was already moved to a permanent location.
    FileAlreadyMoved,
}

impl BodyParserError {
    /// Maps the error variant to its matching HTTP [`StatusCode`].
    #[must_use]
    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::PayloadTooLarge { .. } => StatusCode::PAYLOAD_TOO_LARGE,
            Self::InvalidContentType(_) | Self::UnsupportedEncoding(_) => {
                StatusCode::UNSUPPORTED_MEDIA_TYPE
            }
            Self::MalformedMultipart(_)
            | Self::NestingDepthExceeded { .. }
            | Self::FieldCountExceeded { .. }
            | Self::JsonError(_)
            | Self::FormError(_)
            | Self::Utf8Error(_) => StatusCode::BAD_REQUEST,
            Self::IoError(_) | Self::FileAlreadyMoved => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl fmt::Display for BodyParserError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PayloadTooLarge { limit, received } => {
                write!(
                    formatter,
                    "Payload too large: received {received} bytes, limit is {limit} bytes"
                )
            }
            Self::MalformedMultipart(message) => {
                write!(formatter, "Malformed multipart payload: {message}")
            }
            Self::InvalidContentType(message) => {
                write!(formatter, "Invalid Content-Type header: {message}")
            }
            Self::UnsupportedEncoding(encoding) => {
                write!(formatter, "Unsupported payload encoding: {encoding}")
            }
            Self::NestingDepthExceeded { max_depth } => {
                write!(
                    formatter,
                    "Nesting depth limit of {max_depth} levels exceeded"
                )
            }
            Self::FieldCountExceeded { limit } => {
                write!(formatter, "Maximum field count of {limit} exceeded")
            }
            Self::IoError(message) => write!(formatter, "I/O error during body parsing: {message}"),
            Self::JsonError(message) => write!(formatter, "Invalid JSON payload: {message}"),
            Self::FormError(message) => write!(formatter, "Invalid form payload: {message}"),
            Self::Utf8Error(message) => write!(formatter, "Invalid UTF-8 payload: {message}"),
            Self::FileAlreadyMoved => {
                formatter.write_str("Uploaded file has already been moved to permanent storage")
            }
        }
    }
}

impl Error for BodyParserError {}

impl From<std::io::Error> for BodyParserError {
    fn from(error: std::io::Error) -> Self {
        Self::IoError(error.to_string())
    }
}

impl From<serde_json::Error> for BodyParserError {
    fn from(error: serde_json::Error) -> Self {
        Self::JsonError(error.to_string())
    }
}

impl From<qs::QsError> for BodyParserError {
    fn from(error: qs::QsError) -> Self {
        Self::FormError(error.to_string())
    }
}

impl From<std::str::Utf8Error> for BodyParserError {
    fn from(error: std::str::Utf8Error) -> Self {
        Self::Utf8Error(error.to_string())
    }
}

impl From<BodyParserError> for RouterError {
    fn from(error: BodyParserError) -> Self {
        match error {
            BodyParserError::PayloadTooLarge { limit, received } => Self::PayloadTooLarge {
                max_bytes: limit,
                received_bytes: received,
            },
            BodyParserError::JsonError(msg)
            | BodyParserError::FormError(msg)
            | BodyParserError::Utf8Error(msg)
            | BodyParserError::MalformedMultipart(msg) => Self::DeserializationError(msg),
            other => Self::Internal(other.to_string()),
        }
    }
}
