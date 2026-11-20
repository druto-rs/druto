//! Request-scoped containers for parsed form parameters and uploaded files.

use std::collections::BTreeMap;

use khonj::Value;

use crate::uploaded_file::UploadedFile;

/// Container storing all uploaded files extracted during request body parsing.
#[derive(Debug, Default)]
pub struct UploadedFiles {
    files: Vec<UploadedFile>,
}

impl UploadedFiles {
    /// Creates an empty collection.
    #[must_use]
    pub fn new(files: Vec<UploadedFile>) -> Self {
        Self { files }
    }

    /// Looks up an uploaded file by form field name.
    #[must_use]
    pub fn file(&self, name: &str) -> Option<&UploadedFile> {
        self.files.iter().find(|file| file.field_name() == name)
    }

    /// Looks up a mutable reference to an uploaded file by form field name.
    pub fn file_mut(&mut self, name: &str) -> Option<&mut UploadedFile> {
        self.files.iter_mut().find(|file| file.field_name() == name)
    }

    /// Returns all uploaded files matching the form field name.
    #[must_use]
    pub fn files_by_name(&self, name: &str) -> Vec<&UploadedFile> {
        self.files
            .iter()
            .filter(|file| file.field_name() == name)
            .collect()
    }

    /// Returns a slice of all uploaded files in the request.
    #[must_use]
    pub fn all(&self) -> &[UploadedFile] {
        &self.files
    }

    /// Returns a mutable slice of all uploaded files in the request.
    pub fn all_mut(&mut self) -> &mut [UploadedFile] {
        &mut self.files
    }

    /// Adds an uploaded file to the collection.
    pub fn push(&mut self, file: UploadedFile) {
        self.files.push(file);
    }

    /// Consumes the container and returns the inner vector of uploaded files.
    #[must_use]
    pub fn into_inner(self) -> Vec<UploadedFile> {
        self.files
    }

    /// Returns the total count of uploaded files.
    #[must_use]
    pub fn len(&self) -> usize {
        self.files.len()
    }

    /// Returns `true` if no files were uploaded in the request.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }
}

/// Container storing parsed form fields and structured bracket-notation values.
#[derive(Debug, Clone)]
pub struct FormFields {
    raw_fields: Vec<(String, String)>,
    tree: Value,
}

impl Default for FormFields {
    fn default() -> Self {
        Self {
            raw_fields: Vec::new(),
            tree: Value::Object(BTreeMap::new()),
        }
    }
}

impl FormFields {
    /// Creates a new `FormFields` container from raw pairs and structured [`Value`].
    #[must_use]
    pub fn new(raw_fields: Vec<(String, String)>, tree: Value) -> Self {
        Self { raw_fields, tree }
    }

    /// Looks up a field value by its literal key.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<&str> {
        self.raw_fields
            .iter()
            .rev()
            .find(|(key, _)| key == name)
            .map(|(_, val)| val.as_str())
    }

    /// Returns the parsed hierarchical [`Value`] tree.
    #[must_use]
    pub fn tree(&self) -> &Value {
        &self.tree
    }

    /// Returns all raw field key-value pairs.
    pub fn raw(&self) -> impl Iterator<Item = (&str, &str)> {
        self.raw_fields
            .iter()
            .map(|(key, val)| (key.as_str(), val.as_str()))
    }
}
