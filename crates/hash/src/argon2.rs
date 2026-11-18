//! Argon2id password hashing driver using audited RustCrypto primitives.

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::Error as ArgonError,
};

use crate::{
    error::HashError,
    hasher::{HashAlgorithm, Hasher},
};

/// Configuration options for the Argon2 password hashing driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Argon2Config {
    /// Memory size in KiB (default: 65536 KiB = 64 MiB).
    pub memory: u32,
    /// Number of iterations / passes (default: 3).
    pub iterations: u32,
    /// Number of parallel threads (default: 4).
    pub parallelism: u32,
    /// Argon2 algorithm variant (default: Argon2id).
    pub algorithm: Algorithm,
    /// Argon2 protocol version (default: V0x13).
    pub version: Version,
}

impl Default for Argon2Config {
    fn default() -> Self {
        Self {
            memory: 65536,
            iterations: 3,
            parallelism: 4,
            algorithm: Algorithm::Argon2id,
            version: Version::V0x13,
        }
    }
}

impl Argon2Config {
    /// Creates a configuration with OWASP-recommended default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the memory cost in KiB.
    #[must_use]
    pub fn memory(mut self, memory: u32) -> Self {
        self.memory = memory;
        self
    }

    /// Sets the time cost (number of iterations).
    #[must_use]
    pub fn iterations(mut self, iterations: u32) -> Self {
        self.iterations = iterations;
        self
    }

    /// Sets the degree of parallelism.
    #[must_use]
    pub fn parallelism(mut self, parallelism: u32) -> Self {
        self.parallelism = parallelism;
        self
    }

    /// Validates the configuration and constructs underlying Argon2 parameters.
    pub fn to_params(&self) -> Result<Params, HashError> {
        Params::new(self.memory, self.iterations, self.parallelism, None)
            .map_err(|err| HashError::InvalidParameters(err.to_string()))
    }
}

/// Argon2 password hashing driver supporting Argon2id, Argon2i, and Argon2d.
#[derive(Debug, Clone)]
pub struct Argon2Hasher {
    config: Argon2Config,
    params: Params,
}

impl Default for Argon2Hasher {
    fn default() -> Self {
        Self::new(Argon2Config::default()).expect("default argon2 configuration must be valid")
    }
}

impl Argon2Hasher {
    /// Creates a new Argon2 hasher using the given configuration.
    pub fn new(config: Argon2Config) -> Result<Self, HashError> {
        let params = config.to_params()?;
        Ok(Self { config, params })
    }

    /// Returns a reference to the active configuration.
    #[must_use]
    pub fn config(&self) -> &Argon2Config {
        &self.config
    }

    fn build_argon(&self) -> Argon2<'_> {
        Argon2::new(
            self.config.algorithm,
            self.config.version,
            self.params.clone(),
        )
    }
}

impl Hasher for Argon2Hasher {
    fn algorithm(&self) -> HashAlgorithm {
        match self.config.algorithm {
            Algorithm::Argon2id => HashAlgorithm::Argon2id,
            other => HashAlgorithm::Custom(other.to_string()),
        }
    }

    fn make(&self, password: &str) -> Result<String, HashError> {
        let argon = self.build_argon();
        let password_hash = argon
            .hash_password(password.as_bytes())
            .map_err(|err| HashError::Internal(err.to_string()))?;

        Ok(password_hash.to_string())
    }

    fn verify(&self, password: &str, hashed_value: &str) -> Result<bool, HashError> {
        let parsed = PasswordHash::new(hashed_value)
            .map_err(|err| HashError::InvalidHash(err.to_string()))?;

        if parsed.hash.is_none() {
            return Err(HashError::InvalidHash(
                "missing hash component in PHC string".to_string(),
            ));
        }

        let argon = self.build_argon();
        match argon.verify_password(password.as_bytes(), &parsed) {
            Ok(()) => Ok(true),
            Err(ArgonError::PasswordInvalid) => Ok(false),
            Err(err) => Err(HashError::Internal(err.to_string())),
        }
    }

    fn needs_rehash(&self, hashed_value: &str) -> bool {
        let parsed = match PasswordHash::new(hashed_value) {
            Ok(hash) if hash.hash.is_some() => hash,
            _ => return true,
        };

        if parsed.algorithm.as_str() != self.config.algorithm.as_str() {
            return true;
        }

        let current_version_u32 = u32::from(self.config.version);
        if let Some(version) = parsed.version {
            if version < current_version_u32 {
                return true;
            }
        } else {
            return true;
        }

        let memory_needs_rehash = parsed
            .params
            .get("m")
            .and_then(|value| value.decimal().ok())
            .is_none_or(|memory| memory < self.config.memory);

        if memory_needs_rehash {
            return true;
        }

        let iterations_need_rehash = parsed
            .params
            .get("t")
            .and_then(|value| value.decimal().ok())
            .is_none_or(|iterations| iterations < self.config.iterations);

        if iterations_need_rehash {
            return true;
        }

        parsed
            .params
            .get("p")
            .and_then(|value| value.decimal().ok())
            .is_none_or(|parallelism| parallelism < self.config.parallelism)
    }

    fn is_valid_hash(&self, hashed_value: &str) -> bool {
        match PasswordHash::new(hashed_value) {
            Ok(parsed) => {
                parsed.algorithm.as_str() == self.config.algorithm.as_str() && parsed.hash.is_some()
            }
            Err(_) => false,
        }
    }
}
