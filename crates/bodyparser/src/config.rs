//! Global and per-route configuration for request body parsing.

use std::path::{Path, PathBuf};

use matcher::Method;

use crate::limits::PayloadLimits;

/// Global configuration options for the [`crate::middleware::BodyParserMiddleware`].
#[derive(Clone, Debug)]
pub struct BodyParserConfig {
    allowed_methods: Vec<Method>,
    limits: PayloadLimits,
    auto_process_files: bool,
    process_manually: Vec<String>,
    temp_dir: Option<PathBuf>,
}

impl Default for BodyParserConfig {
    fn default() -> Self {
        Self {
            allowed_methods: vec![
                Method::Post,
                Method::Put,
                Method::Patch,
                Method::Delete,
                Method::Query,
            ],
            limits: PayloadLimits::default(),
            auto_process_files: true,
            process_manually: Vec::new(),
            temp_dir: None,
        }
    }
}

impl BodyParserConfig {
    /// Creates a fluent builder to assemble custom configurations.
    #[must_use]
    pub fn builder() -> BodyParserConfigBuilder {
        BodyParserConfigBuilder::new()
    }

    /// Returns the HTTP methods permitted for body parsing.
    #[must_use]
    pub fn allowed_methods(&self) -> &[Method] {
        &self.allowed_methods
    }

    /// Returns whether the specified HTTP method is configured for body parsing.
    #[must_use]
    pub fn is_method_allowed(&self, method: &Method) -> bool {
        self.allowed_methods.contains(method)
    }

    /// Returns the hierarchical payload limits.
    #[must_use]
    pub fn limits(&self) -> &PayloadLimits {
        &self.limits
    }

    /// Returns whether multipart file parts should be automatically spooled to disk.
    #[must_use]
    pub fn auto_process_files(&self) -> bool {
        self.auto_process_files
    }

    /// Returns route patterns configured for manual multipart streaming.
    #[must_use]
    pub fn process_manually_routes(&self) -> &[String] {
        &self.process_manually
    }

    /// Returns whether the current route path should bypass automatic multipart processing.
    #[must_use]
    pub fn should_process_manually(&self, path: &str) -> bool {
        if !self.auto_process_files {
            return true;
        }

        self.process_manually.iter().any(|pattern| {
            if pattern.ends_with('*') {
                path.starts_with(pattern.trim_end_matches('*'))
            } else {
                path == pattern
            }
        })
    }

    /// Returns the configured temporary directory for file spooling, if specified.
    #[must_use]
    pub fn temp_dir(&self) -> Option<&Path> {
        self.temp_dir.as_deref()
    }
}

/// Fluent builder for [`BodyParserConfig`].
#[derive(Clone, Debug, Default)]
pub struct BodyParserConfigBuilder {
    config: BodyParserConfig,
}

impl BodyParserConfigBuilder {
    /// Creates an empty builder with default configuration baselines.
    #[must_use]
    pub fn new() -> Self {
        Self {
            config: BodyParserConfig::default(),
        }
    }

    /// Sets the HTTP methods that should have their bodies parsed.
    #[must_use]
    pub fn allowed_methods(mut self, methods: Vec<Method>) -> Self {
        self.config.allowed_methods = methods;
        self
    }

    /// Sets the hierarchical payload quota limits.
    #[must_use]
    pub fn limits(mut self, limits: PayloadLimits) -> Self {
        self.config.limits = limits;
        self
    }

    /// Configures whether files in multipart payloads should be automatically spooled to temporary files.
    #[must_use]
    pub fn auto_process_files(mut self, auto_process: bool) -> Self {
        self.config.auto_process_files = auto_process;
        self
    }

    /// Registers a route pattern that should bypass automatic multipart file spooling.
    #[must_use]
    pub fn process_manually(mut self, pattern: impl Into<String>) -> Self {
        self.config.process_manually.push(pattern.into());
        self
    }

    /// Sets a custom filesystem directory for temporary uploaded files.
    #[must_use]
    pub fn temp_dir(mut self, path: impl AsRef<Path>) -> Self {
        self.config.temp_dir = Some(path.as_ref().to_path_buf());
        self
    }

    /// Builds the configured [`BodyParserConfig`].
    #[must_use]
    pub fn build(self) -> BodyParserConfig {
        self.config
    }
}
