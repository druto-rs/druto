//! Streaming request body decoding, multipart/form-data ingestion, temporary file spooling,
//! magic-byte MIME type detection, and payload size quota enforcement for the Druto framework.

pub mod config;
pub mod error;
pub mod form_data;
pub mod limits;
pub mod middleware;
pub mod mime;
pub mod multipart;
pub mod parsers;
pub mod temp_file;
pub mod uploaded_file;
pub mod validator;

pub use config::{BodyParserConfig, BodyParserConfigBuilder};
pub use error::BodyParserError;
pub use form_data::{FormFields, UploadedFiles};
pub use limits::{
    DEFAULT_MAX_BODY_SIZE, DEFAULT_MAX_DEPTH, DEFAULT_MAX_FIELDS_COUNT, DEFAULT_MAX_FIELDS_SIZE,
    DEFAULT_MAX_FILE_SIZE, PayloadLimits, PayloadLimitsBuilder,
};
pub use middleware::BodyParserMiddleware;
pub use mime::{MimeSniffer, OCTET_STREAM_MIME};
pub use multipart::{
    MultipartField, MultipartParser, ParsedMultipart, extract_boundary, parse_multipart_stream,
};
pub use parsers::{parse_form, parse_form_value, parse_json, parse_json_value, parse_text};
pub use temp_file::TempFile;
pub use uploaded_file::UploadedFile;
pub use validator::{FileError, FileValidator};
