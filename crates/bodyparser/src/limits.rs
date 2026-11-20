//! Request payload quota configurations and hierarchical byte limit guards.

use crate::error::BodyParserError;

/// Default global body payload size limit: 20 MB.
pub const DEFAULT_MAX_BODY_SIZE: usize = 20 * 1024 * 1024;

/// Default individual uploaded file size limit: 10 MB.
pub const DEFAULT_MAX_FILE_SIZE: usize = 10 * 1024 * 1024;

/// Default aggregate size limit for all form fields: 1 MB.
pub const DEFAULT_MAX_FIELDS_SIZE: usize = 1024 * 1024;

/// Default maximum number of form fields: 1,000.
pub const DEFAULT_MAX_FIELDS_COUNT: usize = 1000;

/// Default maximum container nesting depth for JSON and form inputs: 100.
pub const DEFAULT_MAX_DEPTH: usize = 100;

/// Hierarchical payload size and field count quota policy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PayloadLimits {
    max_body_size: usize,
    max_file_size: usize,
    max_fields_size: usize,
    max_fields_count: usize,
    max_depth: usize,
}

impl Default for PayloadLimits {
    fn default() -> Self {
        Self {
            max_body_size: DEFAULT_MAX_BODY_SIZE,
            max_file_size: DEFAULT_MAX_FILE_SIZE,
            max_fields_size: DEFAULT_MAX_FIELDS_SIZE,
            max_fields_count: DEFAULT_MAX_FIELDS_COUNT,
            max_depth: DEFAULT_MAX_DEPTH,
        }
    }
}

impl PayloadLimits {
    /// Creates a new `PayloadLimitsBuilder` to assemble custom quotas.
    #[must_use]
    pub fn builder() -> PayloadLimitsBuilder {
        PayloadLimitsBuilder::new()
    }

    /// Returns the global request body ceiling in bytes.
    #[must_use]
    pub fn max_body_size(&self) -> usize {
        self.max_body_size
    }

    /// Returns the maximum allowed size per individual uploaded file in bytes.
    #[must_use]
    pub fn max_file_size(&self) -> usize {
        self.max_file_size
    }

    /// Returns the maximum allowed aggregate size of all text fields in bytes.
    #[must_use]
    pub fn max_fields_size(&self) -> usize {
        self.max_fields_size
    }

    /// Returns the maximum allowed number of fields in a multipart payload.
    #[must_use]
    pub fn max_fields_count(&self) -> usize {
        self.max_fields_count
    }

    /// Returns the maximum allowed container nesting depth.
    #[must_use]
    pub fn max_depth(&self) -> usize {
        self.max_depth
    }

    /// Asserts that total body bytes received do not exceed `max_body_size`.
    pub fn check_body_size(&self, received: usize) -> Result<(), BodyParserError> {
        if received > self.max_body_size {
            return Err(BodyParserError::PayloadTooLarge {
                limit: self.max_body_size,
                received,
            });
        }
        Ok(())
    }

    /// Asserts that individual file bytes received do not exceed `max_file_size`.
    pub fn check_file_size(&self, received: usize) -> Result<(), BodyParserError> {
        if received > self.max_file_size {
            return Err(BodyParserError::PayloadTooLarge {
                limit: self.max_file_size,
                received,
            });
        }
        Ok(())
    }

    /// Asserts that aggregate text field bytes received do not exceed `max_fields_size`.
    pub fn check_fields_size(&self, received: usize) -> Result<(), BodyParserError> {
        if received > self.max_fields_size {
            return Err(BodyParserError::PayloadTooLarge {
                limit: self.max_fields_size,
                received,
            });
        }
        Ok(())
    }

    /// Asserts that total field count does not exceed `max_fields_count`.
    pub fn check_fields_count(&self, count: usize) -> Result<(), BodyParserError> {
        if count > self.max_fields_count {
            return Err(BodyParserError::FieldCountExceeded {
                limit: self.max_fields_count,
            });
        }
        Ok(())
    }

    /// Asserts that container nesting depth does not exceed `max_depth`.
    pub fn check_depth(&self, depth: usize) -> Result<(), BodyParserError> {
        if depth > self.max_depth {
            return Err(BodyParserError::NestingDepthExceeded {
                max_depth: self.max_depth,
            });
        }
        Ok(())
    }
}

/// Fluent builder for [`PayloadLimits`].
#[derive(Clone, Copy, Debug, Default)]
pub struct PayloadLimitsBuilder {
    limits: PayloadLimits,
}

impl PayloadLimitsBuilder {
    /// Creates a builder initialized with default baseline limits.
    #[must_use]
    pub fn new() -> Self {
        Self {
            limits: PayloadLimits::default(),
        }
    }

    /// Sets the maximum global request body size in bytes.
    #[must_use]
    pub fn max_body_size(mut self, bytes: usize) -> Self {
        self.limits.max_body_size = bytes;
        self
    }

    /// Sets the maximum size for an individual uploaded file in bytes.
    #[must_use]
    pub fn max_file_size(mut self, bytes: usize) -> Self {
        self.limits.max_file_size = bytes;
        self
    }

    /// Sets the maximum aggregate size of all form fields in bytes.
    #[must_use]
    pub fn max_fields_size(mut self, bytes: usize) -> Self {
        self.limits.max_fields_size = bytes;
        self
    }

    /// Sets the maximum number of fields allowed in a multipart request.
    #[must_use]
    pub fn max_fields_count(mut self, count: usize) -> Self {
        self.limits.max_fields_count = count;
        self
    }

    /// Sets the maximum container nesting depth for JSON or form inputs.
    #[must_use]
    pub fn max_depth(mut self, depth: usize) -> Self {
        self.limits.max_depth = depth;
        self
    }

    /// Builds the configured [`PayloadLimits`].
    #[must_use]
    pub fn build(self) -> PayloadLimits {
        self.limits
    }
}
