//! Dynamic query value abstract syntax tree representation.

use std::collections::BTreeMap;

/// Dynamic query string value tree node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QsValue {
    /// Represents a key with no value (or explicit null).
    Null,
    /// String scalar value.
    Scalar(String),
    /// Sequential list of values.
    Array(Vec<QsValue>),
    /// Nested dictionary of key-value pairs sorted alphabetically.
    Object(BTreeMap<String, QsValue>),
}

impl QsValue {
    /// Returns `true` if this value is `Null`.
    #[must_use]
    pub const fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    /// Returns the scalar string slice if this value is `Scalar`.
    #[must_use]
    pub fn as_scalar(&self) -> Option<&str> {
        match self {
            Self::Scalar(value) => Some(value),
            _ => None,
        }
    }

    /// Returns the slice of values if this value is `Array`.
    #[must_use]
    pub fn as_array(&self) -> Option<&[Self]> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    /// Returns a mutable reference to the vector of values if this value is `Array`.
    #[must_use]
    pub fn as_array_mut(&mut self) -> Option<&mut Vec<Self>> {
        match self {
            Self::Array(values) => Some(values),
            _ => None,
        }
    }

    /// Returns the map of key-value pairs if this value is `Object`.
    #[must_use]
    pub const fn as_object(&self) -> Option<&BTreeMap<String, Self>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Returns a mutable reference to the map of key-value pairs if this value is `Object`.
    #[must_use]
    pub fn as_object_mut(&mut self) -> Option<&mut BTreeMap<String, Self>> {
        match self {
            Self::Object(map) => Some(map),
            _ => None,
        }
    }

    /// Retrieves a nested property by key if this value is an `Object`.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&Self> {
        match self {
            Self::Object(map) => map.get(key),
            _ => None,
        }
    }

    /// Retrieves an array element by index if this value is an `Array`.
    #[must_use]
    pub fn get_index(&self, index: usize) -> Option<&Self> {
        match self {
            Self::Array(values) => values.get(index),
            _ => None,
        }
    }
}

impl From<&str> for QsValue {
    fn from(value: &str) -> Self {
        Self::Scalar(value.to_string())
    }
}

impl From<String> for QsValue {
    fn from(value: String) -> Self {
        Self::Scalar(value)
    }
}

impl From<Vec<QsValue>> for QsValue {
    fn from(values: Vec<QsValue>) -> Self {
        Self::Array(values)
    }
}

impl From<BTreeMap<String, QsValue>> for QsValue {
    fn from(map: BTreeMap<String, QsValue>) -> Self {
        Self::Object(map)
    }
}
