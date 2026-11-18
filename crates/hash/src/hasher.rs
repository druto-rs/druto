//! Core traits and algorithm identifiers for password hashing drivers.

use std::fmt;

use crate::error::HashError;

/// Supported password hashing algorithms.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum HashAlgorithm {
    /// Argon2id memory-hard hashing algorithm (RFC 9106).
    Argon2id,
    /// Bcrypt modular crypt format hashing algorithm.
    Bcrypt,
    /// Test double algorithm executing without key derivation delay.
    Fake,
    /// User-defined or custom algorithm identifier.
    Custom(String),
}

impl fmt::Display for HashAlgorithm {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Argon2id => write!(formatter, "argon2id"),
            Self::Bcrypt => write!(formatter, "bcrypt"),
            Self::Fake => write!(formatter, "fake"),
            Self::Custom(name) => write!(formatter, "{name}"),
        }
    }
}

/// Common contract implemented by all password hashing drivers.
pub trait Hasher: Send + Sync {
    /// Returns the algorithm identified by this hasher driver.
    fn algorithm(&self) -> HashAlgorithm;

    /// Generates a secure, salted hash string for the given plain text password.
    fn make(&self, password: &str) -> Result<String, HashError>;

    /// Verifies a plain text password against a stored hash string in constant time.
    fn verify(&self, password: &str, hashed_value: &str) -> Result<bool, HashError>;

    /// Checks if the stored hash needs to be rehashed based on updated configuration or algorithm migration.
    fn needs_rehash(&self, hashed_value: &str) -> bool;

    /// Checks if the value matches the valid format for this hasher.
    fn is_valid_hash(&self, hashed_value: &str) -> bool;
}
