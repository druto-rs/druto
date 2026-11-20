//! Uploaded file state encapsulation, validation tracking, and persistence operations.

use std::path::{Path, PathBuf};

use crate::{
    error::BodyParserError,
    temp_file::TempFile,
    validator::{FileError, FileValidator},
};

/// Encapsulates an uploaded file's metadata, temporary storage handle, and validation state.
#[derive(Debug)]
pub struct UploadedFile {
    field_name: String,
    client_filename: String,
    client_content_type: Option<String>,
    detected_mime_type: String,
    extension: Option<String>,
    size: usize,
    temp_file: TempFile,
    errors: Vec<FileError>,
    moved_path: Option<PathBuf>,
}

impl UploadedFile {
    /// Creates a new `UploadedFile` with sanitized client filename and extracted extension.
    pub fn new(
        field_name: impl Into<String>,
        client_filename: &str,
        client_content_type: Option<String>,
        detected_mime_type: impl Into<String>,
        temp_file: TempFile,
    ) -> Self {
        let sanitized = Self::sanitize_filename(client_filename);
        let extension = Path::new(&sanitized)
            .extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| ext.to_ascii_lowercase());

        let size = temp_file.size() as usize;

        Self {
            field_name: field_name.into(),
            client_filename: sanitized,
            client_content_type,
            detected_mime_type: detected_mime_type.into(),
            extension,
            size,
            temp_file,
            errors: Vec::new(),
            moved_path: None,
        }
    }

    /// Returns the form field name associated with this file.
    #[must_use]
    pub fn field_name(&self) -> &str {
        &self.field_name
    }

    /// Returns the sanitized original filename provided by the client.
    #[must_use]
    pub fn client_filename(&self) -> &str {
        &self.client_filename
    }

    /// Returns the client-declared `Content-Type` header value, if supplied.
    #[must_use]
    pub fn client_content_type(&self) -> Option<&str> {
        self.client_content_type.as_deref()
    }

    /// Returns the true MIME type verified via magic-byte inspection.
    #[must_use]
    pub fn detected_mime_type(&self) -> &str {
        &self.detected_mime_type
    }

    /// Returns the lowercase file extension without leading dot, if present.
    #[must_use]
    pub fn extension(&self) -> Option<&str> {
        self.extension.as_deref()
    }

    /// Returns the byte length of the file payload.
    #[must_use]
    pub fn size(&self) -> usize {
        self.size
    }

    /// Returns the current filesystem path to the file (either temporary or permanent destination).
    #[must_use]
    pub fn path(&self) -> &Path {
        if let Some(dest) = &self.moved_path {
            dest.as_path()
        } else {
            self.temp_file.path()
        }
    }

    /// Returns `true` if the file passed all validation checks with zero errors.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.errors.is_empty()
    }

    /// Returns `true` if the file has one or more validation errors.
    #[must_use]
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Returns the list of validation errors recorded against this file.
    #[must_use]
    pub fn errors(&self) -> &[FileError] {
        &self.errors
    }

    /// Records a validation error against this file.
    pub fn add_error(&mut self, error: FileError) {
        self.errors.push(error);
    }

    /// Validates this file against the supplied [`FileValidator`].
    pub fn validate(&mut self, validator: &FileValidator) {
        validator.validate(self);
    }

    /// Moves the file to a permanent storage destination.
    ///
    /// Attempts an atomic rename first, with transparent fallback to copy + delete across mount boundaries.
    pub fn move_to(&mut self, destination: impl AsRef<Path>) -> Result<&Path, BodyParserError> {
        if self.moved_path.is_some() {
            return Err(BodyParserError::FileAlreadyMoved);
        }

        let saved = self.temp_file.persist_to(destination)?;
        self.moved_path = Some(saved);
        Ok(self.moved_path.as_ref().unwrap().as_path())
    }

    /// Returns `true` if the file has been moved to permanent storage.
    #[must_use]
    pub fn is_moved(&self) -> bool {
        self.moved_path.is_some()
    }

    /// Sanitizes an incoming client filename, stripping path traversal sequences (`../`, `..\`) and null bytes.
    #[must_use]
    pub fn sanitize_filename(raw: &str) -> String {
        // 1. Remove null bytes and control characters
        let cleaned: String = raw
            .chars()
            .filter(|char_val| !char_val.is_control() && *char_val != '\0')
            .collect();

        // 2. Extract only the basename (stripping UNIX '/' and Windows '\' directory prefixes)
        let basename = cleaned
            .rsplit('/')
            .next()
            .unwrap_or("")
            .rsplit('\\')
            .next()
            .unwrap_or("");

        // 3. Remove relative directory traversal patterns
        let trimmed = basename.trim_matches('.').trim();

        if trimmed.is_empty() {
            "uploaded_file".to_string()
        } else {
            trimmed.to_string()
        }
    }
}
