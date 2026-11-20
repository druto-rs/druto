//! Runtime-agnostic asynchronous streaming multipart parser.

use std::path::Path;

use bytes::{Bytes, BytesMut};
use futures_util::{Stream, StreamExt};

use crate::{
    error::BodyParserError,
    limits::PayloadLimits,
    multipart::{field::ParsedMultipart, parser::MultipartParser},
};

/// Ingests and parses an asynchronous chunk stream into structured multipart fields and spooled files.
///
/// Operates over any runtime-agnostic [`Stream`] yielding byte chunks. Checks payload size limits
/// incrementally on every chunk, terminating immediately with [`BodyParserError::PayloadTooLarge`]
/// as soon as the limit is breached without waiting for stream completion.
pub async fn parse_multipart_stream<S, E>(
    mut stream: S,
    boundary: &str,
    limits: PayloadLimits,
    temp_dir: Option<&Path>,
) -> Result<ParsedMultipart, BodyParserError>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
    E: std::error::Error + Send + Sync + 'static,
{
    let mut total_bytes = 0;
    let mut buffer = BytesMut::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result
            .map_err(|error| BodyParserError::IoError(format!("Stream reading error: {error}")))?;

        total_bytes += chunk.len();
        limits.check_body_size(total_bytes)?;

        buffer.extend_from_slice(&chunk);
    }

    let mut parser = MultipartParser::new(boundary, limits);
    if let Some(dir) = temp_dir {
        parser = parser.temp_dir(dir);
    }

    parser.parse_slice(&buffer)
}
