//! Central registry for defining and resolving named rate limiting profiles.

use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use crate::middleware::LimiterMiddleware;

/// Central catalog mapping policy profile names to reusable limiter middlewares.
#[derive(Clone, Default)]
pub struct LimiterRegistry {
    entries: Arc<RwLock<HashMap<String, LimiterMiddleware>>>,
}

impl LimiterRegistry {
    /// Creates an empty `LimiterRegistry`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a named rate limiter middleware profile.
    pub fn register(&self, name: impl Into<String>, middleware: LimiterMiddleware) {
        if let Ok(mut map) = self.entries.write() {
            map.insert(name.into(), middleware);
        }
    }

    /// Retrieves a cloned handle to a registered limiter middleware profile.
    #[must_use]
    pub fn get(&self, name: &str) -> Option<LimiterMiddleware> {
        self.entries.read().ok()?.get(name).cloned()
    }

    /// Removes a named rate limiter profile from the registry.
    pub fn remove(&self, name: &str) -> Option<LimiterMiddleware> {
        self.entries.write().ok()?.remove(name)
    }

    /// Returns the number of registered rate limiter profiles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.read().map(|m| m.len()).unwrap_or(0)
    }

    /// Returns `true` if the registry contains no limiter profiles.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
