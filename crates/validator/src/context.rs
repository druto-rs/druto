//! Ambient runtime metadata context passed to validation rules.

use std::{
    any::{Any, TypeId},
    collections::HashMap,
    fmt,
    sync::Arc,
};

/// Type-safe container storing ambient metadata for validation rules.
///
/// `ValidationContext` allows route handlers, services, and background workers to supply
/// ambient state (such as the authenticated user ID, tenant ID, or request IP) to custom
/// validation rules and refinement closures without coupling schemas to specific environments.
#[derive(Clone, Default)]
pub struct ValidationContext {
    entries: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl ValidationContext {
    /// Creates a new empty `ValidationContext`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Inserts a typed value into the context.
    ///
    /// If an entry of the same type already exists, it is overwritten.
    pub fn insert<T: Send + Sync + 'static>(&mut self, value: T) {
        self.entries.insert(TypeId::of::<T>(), Arc::new(value));
    }

    /// Returns a reference to a typed value if present in the context.
    #[must_use]
    pub fn get<T: 'static>(&self) -> Option<&T> {
        self.entries
            .get(&TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref::<T>())
    }

    /// Returns `true` if an entry of type `T` exists in the context.
    #[must_use]
    pub fn contains<T: 'static>(&self) -> bool {
        self.entries.contains_key(&TypeId::of::<T>())
    }

    /// Removes an entry of type `T` from the context, returning true if it was present.
    pub fn remove<T: 'static>(&mut self) -> bool {
        self.entries.remove(&TypeId::of::<T>()).is_some()
    }

    /// Returns the number of items stored in the context.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Returns `true` if the context contains no entries.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clears all entries from the context.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

impl fmt::Debug for ValidationContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ValidationContext")
            .field("entry_count", &self.entries.len())
            .finish()
    }
}
