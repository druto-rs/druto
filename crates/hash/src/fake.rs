//! Fast deterministic test double hasher for integration and unit tests.

use subtle::ConstantTimeEq;

use crate::{
    error::HashError,
    hasher::{HashAlgorithm, Hasher},
};

/// Test double hasher designed to execute without deliberate CPU delay.
///
/// Plaintext inputs are encoded with a `$fake$` prefix so that test assertions
/// and migration hooks can distinguish hashed values from unhashed passwords.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct FakeHasher;

impl FakeHasher {
    /// Creates a new fake hasher instance.
    #[must_use]
    pub fn new() -> Self {
        Self
    }
}

impl Hasher for FakeHasher {
    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Fake
    }

    fn make(&self, password: &str) -> Result<String, HashError> {
        Ok(format!("$fake${password}"))
    }

    fn verify(&self, password: &str, hashed_value: &str) -> Result<bool, HashError> {
        let candidate = if let Some(stripped) = hashed_value.strip_prefix("$fake$") {
            stripped
        } else {
            hashed_value
        };

        let candidate_bytes = candidate.as_bytes();
        let password_bytes = password.as_bytes();

        let is_equal = if candidate_bytes.len() == password_bytes.len() {
            candidate_bytes.ct_eq(password_bytes).into()
        } else {
            false
        };

        Ok(is_equal)
    }

    fn needs_rehash(&self, hashed_value: &str) -> bool {
        !hashed_value.starts_with("$fake$")
    }

    fn is_valid_hash(&self, hashed_value: &str) -> bool {
        hashed_value.starts_with("$fake$")
    }
}
