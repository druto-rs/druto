//! Multi-driver hashing manager and facade with scoped test mocking.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::{
    argon2::Argon2Hasher, bcrypt::BcryptHasher, error::HashError, fake::FakeHasher, hasher::Hasher,
};

/// Builder for constructing a configured [`HashManager`].
#[derive(Default)]
pub struct HashManagerBuilder {
    drivers: HashMap<String, Arc<dyn Hasher>>,
    default_driver: Option<String>,
}

impl HashManagerBuilder {
    /// Creates a new empty builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a hasher driver under a specified identifier.
    #[must_use]
    pub fn driver<H: Hasher + 'static>(mut self, name: impl Into<String>, hasher: H) -> Self {
        self.drivers.insert(name.into(), Arc::new(hasher));
        self
    }

    /// Registers a pre-boxed or arc-wrapped hasher driver.
    #[must_use]
    pub fn driver_arc(mut self, name: impl Into<String>, hasher: Arc<dyn Hasher>) -> Self {
        self.drivers.insert(name.into(), hasher);
        self
    }

    /// Sets the name of the default driver.
    #[must_use]
    pub fn default_driver(mut self, name: impl Into<String>) -> Self {
        self.default_driver = Some(name.into());
        self
    }

    /// Builds the configured [`HashManager`].
    pub fn build(self) -> Result<HashManager, HashError> {
        let default_name = self
            .default_driver
            .unwrap_or_else(|| "argon2id".to_string());

        if !self.drivers.contains_key(&default_name) {
            return Err(HashError::DriverNotFound(format!(
                "default driver '{default_name}' is not registered in the driver list"
            )));
        }

        Ok(HashManager {
            drivers: Arc::new(self.drivers),
            default_driver: default_name,
            fake_override: Arc::new(RwLock::new(None)),
        })
    }
}

/// Central registry managing multiple password hashing drivers.
#[derive(Clone)]
pub struct HashManager {
    drivers: Arc<HashMap<String, Arc<dyn Hasher>>>,
    default_driver: String,
    fake_override: Arc<RwLock<Option<Arc<dyn Hasher>>>>,
}

impl Default for HashManager {
    fn default() -> Self {
        let mut drivers: HashMap<String, Arc<dyn Hasher>> = HashMap::new();
        let argon = Arc::new(Argon2Hasher::default());
        let bcrypt = Arc::new(BcryptHasher::default());
        let fake = Arc::new(FakeHasher::new());

        drivers.insert("argon2id".to_string(), argon.clone());
        drivers.insert("argon".to_string(), argon);
        drivers.insert("bcrypt".to_string(), bcrypt);
        drivers.insert("fake".to_string(), fake);

        Self {
            drivers: Arc::new(drivers),
            default_driver: "argon2id".to_string(),
            fake_override: Arc::new(RwLock::new(None)),
        }
    }
}

impl HashManager {
    /// Creates a new builder for configuring a custom manager.
    #[must_use]
    pub fn builder() -> HashManagerBuilder {
        HashManagerBuilder::new()
    }

    /// Creates a standalone manager where the default driver is [`FakeHasher`].
    #[must_use]
    pub fn fake_manager() -> Self {
        let mut drivers: HashMap<String, Arc<dyn Hasher>> = HashMap::new();
        drivers.insert("fake".to_string(), Arc::new(FakeHasher::new()));

        Self {
            drivers: Arc::new(drivers),
            default_driver: "fake".to_string(),
            fake_override: Arc::new(RwLock::new(None)),
        }
    }

    /// Enables fake hashing mode and returns an RAII guard that restores normal hashing upon drop.
    pub fn fake(&self) -> FakeGuard {
        let fake_hasher: Arc<dyn Hasher> = Arc::new(FakeHasher::new());
        let mut guard = self
            .fake_override
            .write()
            .expect("hash manager lock poisoned during fake activation");

        let previous = guard.replace(fake_hasher);
        FakeGuard {
            manager: self.clone(),
            previous,
        }
    }

    /// Disables fake hashing mode immediately.
    pub fn restore(&self) {
        let mut guard = self
            .fake_override
            .write()
            .expect("hash manager lock poisoned during restore");
        guard.take();
    }

    /// Resolves a driver by its registered name.
    pub fn driver(&self, name: &str) -> Result<Arc<dyn Hasher>, HashError> {
        let guard = self
            .fake_override
            .read()
            .expect("hash manager lock poisoned");

        if let Some(fake) = guard.as_ref() {
            if !self.drivers.contains_key(name) {
                return Err(HashError::DriverNotFound(name.to_string()));
            }
            return Ok(fake.clone());
        }

        self.drivers
            .get(name)
            .cloned()
            .ok_or_else(|| HashError::DriverNotFound(name.to_string()))
    }

    /// Resolves a driver by its registered name (alias for `driver`).
    pub fn use_driver(&self, name: &str) -> Result<Arc<dyn Hasher>, HashError> {
        self.driver(name)
    }

    /// Returns the active default hasher driver, respecting active fake test doubles.
    #[must_use]
    pub fn default_hasher(&self) -> Arc<dyn Hasher> {
        let guard = self
            .fake_override
            .read()
            .expect("hash manager lock poisoned");
        if let Some(fake) = guard.as_ref() {
            return fake.clone();
        }

        self.drivers
            .get(&self.default_driver)
            .cloned()
            .expect("default hasher must exist in registered drivers")
    }

    /// Generates a hash for the plain text value using the active default driver.
    pub fn make(&self, password: &str) -> Result<String, HashError> {
        self.default_hasher().make(password)
    }

    /// Verifies the plain text value against a stored hash string using the active default driver.
    pub fn verify(&self, password: &str, hashed_value: &str) -> Result<bool, HashError> {
        self.default_hasher().verify(password, hashed_value)
    }

    /// Checks if the stored hash string needs rehashing using the active default driver.
    #[must_use]
    pub fn needs_rehash(&self, hashed_value: &str) -> bool {
        self.default_hasher().needs_rehash(hashed_value)
    }

    /// Checks if the stored hash matches the format expected by the active default driver.
    #[must_use]
    pub fn is_valid_hash(&self, hashed_value: &str) -> bool {
        self.default_hasher().is_valid_hash(hashed_value)
    }
}

/// RAII guard that restores the previous hashing state when dropped.
pub struct FakeGuard {
    manager: HashManager,
    previous: Option<Arc<dyn Hasher>>,
}

impl Drop for FakeGuard {
    fn drop(&mut self) {
        let mut guard = self
            .manager
            .fake_override
            .write()
            .expect("hash manager lock poisoned during fake cleanup");
        *guard = self.previous.take();
    }
}
