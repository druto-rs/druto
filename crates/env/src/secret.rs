use std::fmt;

/// A wrapper type for sensitive values (passwords, tokens, API keys).
///
/// `Secret<T>` prevents sensitive values from being accidentally printed in
/// logs, debug traces, or error diagnostics by masking them as `[REDACTED]`.
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Secret<T> {
    inner: T,
}

impl<T> Secret<T> {
    /// Wraps a value in a secret container.
    pub fn new(inner: T) -> Self {
        Self { inner }
    }

    /// Exposes a reference to the sensitive value.
    pub fn expose_secret(&self) -> &T {
        &self.inner
    }

    /// Consumes the wrapper and returns the underlying value.
    pub fn into_inner(self) -> T {
        self.inner
    }
}

impl<T> From<T> for Secret<T> {
    fn from(value: T) -> Self {
        Self::new(value)
    }
}

impl<T> fmt::Debug for Secret<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

impl<T> fmt::Display for Secret<T> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("[REDACTED]")
    }
}

#[cfg(feature = "serde")]
impl<T> serde::Serialize for Secret<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str("[REDACTED]")
    }
}

#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Secret<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        T::deserialize(deserializer).map(Self::new)
    }
}
