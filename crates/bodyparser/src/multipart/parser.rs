//! Self-contained streaming multipart boundary state machine (RFC 7578 / RFC 2046).

use std::path::Path;

use bytes::{Buf, BytesMut};

use crate::{
    error::BodyParserError, limits::PayloadLimits, mime::MimeSniffer,
    multipart::field::ParsedMultipart, temp_file::TempFile, uploaded_file::UploadedFile,
};

/// Extracts the boundary parameter from an HTTP `Content-Type` header string.
pub fn extract_boundary(content_type: &str) -> Result<String, BodyParserError> {
    for part in content_type.split(';') {
        let trimmed = part.trim();
        if let Some(boundary_param) = trimmed.strip_prefix("boundary=") {
            let clean = boundary_param.trim().trim_matches('"').trim();
            if clean.is_empty() {
                return Err(BodyParserError::InvalidContentType(
                    "Empty multipart boundary parameter".to_string(),
                ));
            }
            return Ok(clean.to_string());
        }
    }

    Err(BodyParserError::InvalidContentType(
        "Missing boundary parameter in multipart Content-Type header".to_string(),
    ))
}

/// Parsed header fields for an individual multipart body part.
#[derive(Debug, Default)]
struct PartHeaders {
    name: String,
    filename: Option<String>,
    content_type: Option<String>,
}

impl PartHeaders {
    /// Parses RFC 7578 headers from raw header lines.
    fn parse(header_block: &str) -> Result<Self, BodyParserError> {
        let mut name = None;
        let mut filename = None;
        let mut content_type = None;

        for line in header_block.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            if let Some((header_name, header_value)) = line.split_once(':') {
                let header_name = header_name.trim();
                let header_value = header_value.trim();

                if header_name.eq_ignore_ascii_case("content-disposition") {
                    for param in header_value.split(';') {
                        let param = param.trim();
                        if let Some(name_val) = param.strip_prefix("name=") {
                            name = Some(name_val.trim_matches('"').to_string());
                        } else if let Some(filename_val) = param.strip_prefix("filename=") {
                            filename = Some(filename_val.trim_matches('"').to_string());
                        } else if let Some(filename_star) = param.strip_prefix("filename*=") {
                            // RFC 5987 encoded filename: utf-8''filename.ext
                            let encoded = filename_star.trim_matches('"');
                            if let Some((_, raw_name)) = encoded.split_once("''") {
                                filename = Some(raw_name.to_string());
                            } else {
                                filename = Some(encoded.to_string());
                            }
                        }
                    }
                } else if header_name.eq_ignore_ascii_case("content-type") {
                    content_type = Some(header_value.to_string());
                }
            }
        }

        let name = name.ok_or_else(|| {
            BodyParserError::MalformedMultipart(
                "Missing 'name' in Content-Disposition header".to_string(),
            )
        })?;

        Ok(Self {
            name,
            filename,
            content_type,
        })
    }
}

/// Self-contained parser state machine for processing chunked `multipart/form-data` streams.
pub struct MultipartParser {
    boundary: String,
    delimiter: Vec<u8>,
    limits: PayloadLimits,
    temp_dir: Option<std::path::PathBuf>,
}

impl MultipartParser {
    /// Constructs a parser initialized with the specified boundary and limits.
    #[must_use]
    pub fn new(boundary: impl Into<String>, limits: PayloadLimits) -> Self {
        let b = boundary.into();
        let delimiter = format!("--{b}").into_bytes();

        Self {
            boundary: b,
            delimiter,
            limits,
            temp_dir: None,
        }
    }

    /// Sets a custom directory for temporary file spooling.
    #[must_use]
    pub fn temp_dir(mut self, dir: impl AsRef<Path>) -> Self {
        self.temp_dir = Some(dir.as_ref().to_path_buf());
        self
    }

    /// Returns the boundary string used by this parser.
    #[must_use]
    pub fn boundary(&self) -> &str {
        &self.boundary
    }

    /// Parses an entire multipart payload from an in-memory buffer.
    pub fn parse_slice(&self, payload: &[u8]) -> Result<ParsedMultipart, BodyParserError> {
        self.limits.check_body_size(payload.len())?;

        let mut result = ParsedMultipart::new();
        let mut buffer = BytesMut::from(payload);

        let mut field_count = 0;
        let mut aggregate_fields_size = 0;

        // 1. Locate initial boundary delimiter
        let start_pos = match find_subsequence(&buffer, &self.delimiter) {
            Some(pos) => pos,
            None => {
                return Err(BodyParserError::MalformedMultipart(
                    "Starting boundary delimiter not found".to_string(),
                ));
            }
        };

        // Advance buffer past initial boundary
        buffer.advance(start_pos + self.delimiter.len());

        // 2. Iterate through body parts until closing delimiter or EOF
        loop {
            // Skip CRLF or LF after delimiter
            Self::consume_newline(&mut buffer);

            // Check if this is the closing delimiter
            if buffer.starts_with(b"--") {
                // End of multipart payload
                break;
            }

            // Check field count limit
            field_count += 1;
            self.limits.check_fields_count(field_count)?;

            // 3. Locate end of headers (marked by \r\n\r\n or \n\n)
            let header_end_pos = match find_header_terminator(&buffer) {
                Some(pos) => pos,
                None => {
                    return Err(BodyParserError::MalformedMultipart(
                        "Part header terminator not found".to_string(),
                    ));
                }
            };

            let header_bytes = buffer.split_to(header_end_pos);
            let header_str = std::str::from_utf8(&header_bytes).map_err(|_| {
                BodyParserError::MalformedMultipart(
                    "Part headers contain invalid UTF-8".to_string(),
                )
            })?;

            let headers = PartHeaders::parse(header_str)?;

            // Advance past the double newline terminator
            Self::consume_header_terminator(&mut buffer);

            // 4. Locate next boundary delimiter
            let next_boundary_pos = match find_subsequence(&buffer, &self.delimiter) {
                Some(pos) => pos,
                None => {
                    return Err(BodyParserError::MalformedMultipart(
                        "Closing boundary delimiter for part not found".to_string(),
                    ));
                }
            };

            // Part body ends before the preceding CRLF/LF
            let body_bytes_len = if next_boundary_pos >= 2
                && &buffer[next_boundary_pos - 2..next_boundary_pos] == b"\r\n"
            {
                next_boundary_pos - 2
            } else if next_boundary_pos >= 1 && buffer[next_boundary_pos - 1] == b'\n' {
                next_boundary_pos - 1
            } else {
                next_boundary_pos
            };

            let part_body = buffer.split_to(body_bytes_len);

            // Advance buffer to boundary position
            let remaining_advance = next_boundary_pos - body_bytes_len;
            buffer.advance(remaining_advance + self.delimiter.len());

            // 5. Ingest part as file or text field
            if let Some(filename) = headers.filename {
                // Individual file size check
                self.limits.check_file_size(part_body.len())?;

                let mut temp_file = TempFile::new(self.temp_dir.as_deref())?;
                temp_file.write_chunk(&part_body)?;
                temp_file.flush()?;

                // Sniff initial bytes for true MIME type
                let sniff_bytes = temp_file.read_initial_bytes(512)?;
                let detected_mime = MimeSniffer::sniff_or_fallback(&sniff_bytes);

                let uploaded_file = UploadedFile::new(
                    headers.name,
                    &filename,
                    headers.content_type,
                    detected_mime,
                    temp_file,
                );

                result.add_file(uploaded_file);
            } else {
                // Text field
                aggregate_fields_size += part_body.len();
                self.limits.check_fields_size(aggregate_fields_size)?;

                let text_val = std::str::from_utf8(&part_body).map_err(|_| {
                    BodyParserError::MalformedMultipart(
                        "Text field contains invalid UTF-8".to_string(),
                    )
                })?;

                result.add_text_field(headers.name, text_val);
            }
        }

        Ok(result)
    }

    /// Advances buffer past CRLF (`\r\n`) or LF (`\n`).
    fn consume_newline(buffer: &mut BytesMut) {
        if buffer.starts_with(b"\r\n") {
            buffer.advance(2);
        } else if buffer.starts_with(b"\n") {
            buffer.advance(1);
        }
    }

    /// Advances buffer past `\r\n\r\n`, `\n\n`, `\r\n`, or `\n`.
    fn consume_header_terminator(buffer: &mut BytesMut) {
        if buffer.starts_with(b"\r\n\r\n") {
            buffer.advance(4);
        } else if buffer.starts_with(b"\n\n") || buffer.starts_with(b"\r\n") {
            buffer.advance(2);
        } else if buffer.starts_with(b"\n") {
            buffer.advance(1);
        }
    }
}

/// Finds the starting byte index of `needle` within `haystack`.
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || haystack.len() < needle.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

/// Locates the byte index before the header-body terminator (`\r\n\r\n` or `\n\n`).
fn find_header_terminator(buffer: &[u8]) -> Option<usize> {
    for idx in 0..buffer.len() {
        if buffer[idx..].starts_with(b"\r\n\r\n") {
            return Some(idx);
        }
        if buffer[idx..].starts_with(b"\n\n") {
            return Some(idx);
        }
    }
    None
}
