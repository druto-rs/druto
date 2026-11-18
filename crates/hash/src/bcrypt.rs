//! Bcrypt password hashing driver supporting Modular Crypt Format.

use bcrypt::DEFAULT_COST;

use crate::{
    error::HashError,
    hasher::{HashAlgorithm, Hasher},
};

/// Minimum cost rounds supported by Bcrypt.
pub const MIN_COST: u32 = 4;
/// Maximum cost rounds supported by Bcrypt.
pub const MAX_COST: u32 = 31;

/// Configuration options for the Bcrypt password hashing driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BcryptConfig {
    /// Cost factor (number of rounds, between 4 and 31).
    pub rounds: u32,
}

impl Default for BcryptConfig {
    fn default() -> Self {
        Self {
            rounds: DEFAULT_COST,
        }
    }
}

impl BcryptConfig {
    /// Creates a new configuration with standard default cost rounds (12).
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the cost rounds factor.
    pub fn rounds(mut self, rounds: u32) -> Result<Self, HashError> {
        if !(MIN_COST..=MAX_COST).contains(&rounds) {
            return Err(HashError::InvalidParameters(format!(
                "bcrypt cost rounds must be between {MIN_COST} and {MAX_COST}, got {rounds}"
            )));
        }
        self.rounds = rounds;
        Ok(self)
    }
}

/// Bcrypt password hashing driver implementing standard Modular Crypt Format.
#[derive(Debug, Clone)]
pub struct BcryptHasher {
    config: BcryptConfig,
}

impl Default for BcryptHasher {
    fn default() -> Self {
        Self::new(BcryptConfig::default()).expect("default bcrypt config must be valid")
    }
}

impl BcryptHasher {
    /// Creates a new Bcrypt hasher using the given configuration.
    pub fn new(config: BcryptConfig) -> Result<Self, HashError> {
        if !(MIN_COST..=MAX_COST).contains(&config.rounds) {
            return Err(HashError::InvalidParameters(format!(
                "bcrypt cost rounds must be between {MIN_COST} and {MAX_COST}, got {}",
                config.rounds
            )));
        }
        Ok(Self { config })
    }

    /// Returns a reference to the active configuration.
    #[must_use]
    pub fn config(&self) -> &BcryptConfig {
        &self.config
    }
}

impl Hasher for BcryptHasher {
    fn algorithm(&self) -> HashAlgorithm {
        HashAlgorithm::Bcrypt
    }

    fn make(&self, password: &str) -> Result<String, HashError> {
        bcrypt::hash(password, self.config.rounds)
            .map_err(|err| HashError::Internal(err.to_string()))
    }

    fn verify(&self, password: &str, hashed_value: &str) -> Result<bool, HashError> {
        match bcrypt::verify(password, hashed_value) {
            Ok(is_valid) => Ok(is_valid),
            Err(bcrypt::BcryptError::InvalidHash(reason)) => {
                Err(HashError::InvalidHash(reason.to_string()))
            }
            Err(bcrypt::BcryptError::CostNotAllowed(cost)) => {
                Err(HashError::InvalidHash(format!("cost {cost} not allowed")))
            }
            Err(err) => Err(HashError::Internal(err.to_string())),
        }
    }

    fn needs_rehash(&self, hashed_value: &str) -> bool {
        let parts: Vec<&str> = hashed_value.split('$').collect();

        // Standard bcrypt format: $2b$12$salt_and_hash (4 elements when split on $)
        if parts.len() != 4 || !parts[0].is_empty() {
            return true;
        }

        let version = parts[1];
        let is_valid_version = matches!(version, "2a" | "2b" | "2y" | "2x");
        if !is_valid_version {
            return true;
        }

        let cost_result = parts[2].parse::<u32>();
        match cost_result {
            Ok(stored_rounds) => stored_rounds < self.config.rounds,
            Err(_) => true,
        }
    }

    fn is_valid_hash(&self, hashed_value: &str) -> bool {
        let parts: Vec<&str> = hashed_value.split('$').collect();
        if parts.len() != 4 || !parts[0].is_empty() {
            return false;
        }

        let is_valid_version = matches!(parts[1], "2a" | "2b" | "2y" | "2x");
        if !is_valid_version {
            return false;
        }

        parts[2].parse::<u32>().is_ok() && parts[3].len() == 53
    }
}
