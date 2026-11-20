//! Validation rules and constraints for uploaded files.

use std::fmt;

use crate::uploaded_file::UploadedFile;

/// Error conditions encountered during file-level validation checks.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileError {
    /// File byte size exceeded the configured maximum size.
    SizeLimitExceeded {
        /// Configured size ceiling in bytes.
        limit: usize,
        /// Actual file size in bytes.
        actual: usize,
    },
    /// File byte size was below the configured minimum size.
    SizeBelowMinimum {
        /// Configured minimum size in bytes.
        minimum: usize,
        /// Actual file size in bytes.
        actual: usize,
    },
    /// File extension was not in the allowed extensions whitelist.
    InvalidExtension {
        /// The file's extension.
        extension: String,
        /// Whitelisted extensions.
        allowed: Vec<String>,
    },
    /// File MIME type was not in the allowed MIME types whitelist.
    InvalidMimeType {
        /// The file's detected or declared MIME type.
        mime: String,
        /// Whitelisted MIME types.
        allowed: Vec<String>,
    },
    /// Detected magic-byte MIME type conflicted with client-declared type or extension.
    MimeMismatch {
        /// Client-declared MIME type.
        declared: String,
        /// True MIME type detected via magic-byte inspection.
        detected: String,
    },
}

impl fmt::Display for FileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SizeLimitExceeded { limit, actual } => {
                write!(
                    formatter,
                    "File size of {actual} bytes exceeds maximum limit of {limit} bytes"
                )
            }
            Self::SizeBelowMinimum { minimum, actual } => {
                write!(
                    formatter,
                    "File size of {actual} bytes is below required minimum of {minimum} bytes"
                )
            }
            Self::InvalidExtension { extension, allowed } => {
                write!(
                    formatter,
                    "File extension '{extension}' is not allowed. Allowed extensions: {}",
                    allowed.join(", ")
                )
            }
            Self::InvalidMimeType { mime, allowed } => {
                write!(
                    formatter,
                    "MIME type '{mime}' is not allowed. Allowed types: {}",
                    allowed.join(", ")
                )
            }
            Self::MimeMismatch { declared, detected } => {
                write!(
                    formatter,
                    "Detected content signature '{detected}' does not match declared type '{declared}'"
                )
            }
        }
    }
}

impl std::error::Error for FileError {}

/// Fluent validator enforcing size, extension, and MIME constraints on uploaded files.
#[derive(Clone, Debug, Default)]
pub struct FileValidator {
    allowed_extensions: Option<Vec<String>>,
    allowed_mimes: Option<Vec<String>>,
    max_size: Option<usize>,
    min_size: Option<usize>,
}

impl FileValidator {
    /// Creates an empty validator with no constraints.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a single allowed file extension (e.g. `"png"` or `".png"`).
    #[must_use]
    pub fn extension(mut self, extension: impl Into<String>) -> Self {
        let clean = extension
            .into()
            .trim_start_matches('.')
            .to_ascii_lowercase();
        self.allowed_extensions
            .get_or_insert_with(Vec::new)
            .push(clean);
        self
    }

    /// Whitelists a collection of file extensions.
    #[must_use]
    pub fn extensions<I, S>(mut self, extensions: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for ext in extensions {
            self = self.extension(ext);
        }
        self
    }

    /// Adds a single allowed MIME type (e.g. `"image/png"`).
    #[must_use]
    pub fn mime(mut self, mime_type: impl Into<String>) -> Self {
        self.allowed_mimes
            .get_or_insert_with(Vec::new)
            .push(mime_type.into().to_ascii_lowercase());
        self
    }

    /// Whitelists a collection of MIME types.
    #[must_use]
    pub fn mimes<I, S>(mut self, mime_types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        for mime in mime_types {
            self = self.mime(mime);
        }
        self
    }

    /// Sets the maximum allowable file size in bytes.
    #[must_use]
    pub fn max_size(mut self, bytes: usize) -> Self {
        self.max_size = Some(bytes);
        self
    }

    /// Sets the minimum required file size in bytes.
    #[must_use]
    pub fn min_size(mut self, bytes: usize) -> Self {
        self.min_size = Some(bytes);
        self
    }

    /// Validates an [`UploadedFile`], appending any constraint violations to the file's error list.
    pub fn validate(&self, file: &mut UploadedFile) {
        // 1. Max size check
        if let Some(max) = self.max_size
            && file.size() > max
        {
            file.add_error(FileError::SizeLimitExceeded {
                limit: max,
                actual: file.size(),
            });
        }

        // 2. Min size check
        if let Some(min) = self.min_size
            && file.size() < min
        {
            file.add_error(FileError::SizeBelowMinimum {
                minimum: min,
                actual: file.size(),
            });
        }

        // 3. Extension check
        if let Some(allowed_exts) = &self.allowed_extensions {
            let matches = file
                .extension()
                .map(|ext| {
                    allowed_exts
                        .iter()
                        .any(|allowed| allowed.eq_ignore_ascii_case(ext))
                })
                .unwrap_or(false);

            if !matches {
                file.add_error(FileError::InvalidExtension {
                    extension: file.extension().unwrap_or("").to_string(),
                    allowed: allowed_exts.clone(),
                });
            }
        }

        // 4. MIME check
        if let Some(allowed_mimes) = &self.allowed_mimes {
            let detected = file.detected_mime_type();
            let matches = allowed_mimes.iter().any(|allowed| {
                if allowed.ends_with("/*") {
                    let prefix = allowed.trim_end_matches("/*");
                    detected.starts_with(prefix)
                } else {
                    allowed.eq_ignore_ascii_case(detected)
                }
            });

            if !matches {
                file.add_error(FileError::InvalidMimeType {
                    mime: detected.to_string(),
                    allowed: allowed_mimes.clone(),
                });
            }
        }
    }
}
