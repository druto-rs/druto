//! Multipart field representations and structured form collections.

use std::collections::BTreeMap;

use qs::{
    QsValue,
    encoding::{encode_component, encode_key_preserving_brackets},
};

use crate::{error::BodyParserError, form_data::UploadedFiles, uploaded_file::UploadedFile};

/// An individual field extracted from a multipart payload.
#[derive(Debug)]
pub enum MultipartField {
    /// A text form field.
    Text {
        /// Form field name.
        name: String,
        /// Form field text value.
        value: String,
    },
    /// An uploaded binary file.
    File(UploadedFile),
}

/// The collection of parsed text fields and uploaded files extracted from a multipart payload.
#[derive(Debug, Default)]
pub struct ParsedMultipart {
    text_fields: Vec<(String, String)>,
    files: UploadedFiles,
}

impl ParsedMultipart {
    /// Creates an empty collection.
    #[must_use]
    pub fn new() -> Self {
        Self {
            text_fields: Vec::new(),
            files: UploadedFiles::default(),
        }
    }

    /// Adds a text field to the collection.
    pub fn add_text_field(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.text_fields.push((name.into(), value.into()));
    }

    /// Adds an uploaded file to the collection.
    pub fn add_file(&mut self, file: UploadedFile) {
        self.files.push(file);
    }

    /// Returns an iterator over all raw text field key-value pairs.
    pub fn text_fields(&self) -> impl Iterator<Item = (&str, &str)> {
        self.text_fields
            .iter()
            .map(|(key, val)| (key.as_str(), val.as_str()))
    }

    /// Looks up a single text field value by name (returns the last one if repeated).
    #[must_use]
    pub fn text(&self, name: &str) -> Option<&str> {
        self.text_fields
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, val)| val.as_str())
    }

    /// Looks up an uploaded file by its field name.
    #[must_use]
    pub fn file(&self, name: &str) -> Option<&UploadedFile> {
        self.files.file(name)
    }

    /// Looks up a mutable reference to an uploaded file by its field name.
    pub fn file_mut(&mut self, name: &str) -> Option<&mut UploadedFile> {
        self.files.file_mut(name)
    }

    /// Returns all uploaded files matching the given field name (e.g. for multiple file inputs).
    #[must_use]
    pub fn files_by_name(&self, name: &str) -> Vec<&UploadedFile> {
        self.files.files_by_name(name)
    }

    /// Returns a slice of all uploaded files.
    #[must_use]
    pub fn all_files(&self) -> &[UploadedFile] {
        self.files.all()
    }

    /// Returns a mutable slice of all uploaded files.
    pub fn all_files_mut(&mut self) -> &mut [UploadedFile] {
        self.files.all_mut()
    }

    /// Consumes the container and returns all uploaded files.
    #[must_use]
    pub fn into_files(self) -> Vec<UploadedFile> {
        self.files.into_inner()
    }

    /// Structures all text fields into nested maps and arrays using Druto's [`qs`] parser.
    pub fn structured_fields(&self) -> Result<QsValue, BodyParserError> {
        if self.text_fields.is_empty() {
            return Ok(QsValue::Object(BTreeMap::new()));
        }

        // Encode key-value pairs into query string representation for qs parsing
        let encoded_query = self
            .text_fields
            .iter()
            .map(|(key, value)| {
                let encoded_key = encode_key_preserving_brackets(key);
                let encoded_val = encode_component(value);
                format!("{encoded_key}={encoded_val}")
            })
            .collect::<Vec<_>>()
            .join("&");

        qs::parse(&encoded_query).map_err(BodyParserError::from)
    }
}
