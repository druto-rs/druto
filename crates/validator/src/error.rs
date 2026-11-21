//! Hierarchical field-level error types, rule parameters, and RFC-compliant error reporting.

use std::{collections::BTreeMap, fmt};

use serde::{Deserialize, Serialize};

/// Key-value parameters associated with a validation rule failure.
///
/// Used for template placeholder interpolation (such as `:min`, `:max`, `:field`, `:expected`).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct RuleParams {
    params: BTreeMap<String, String>,
}

impl RuleParams {
    /// Creates an empty set of rule parameters.
    #[must_use]
    pub fn new() -> Self {
        Self {
            params: BTreeMap::new(),
        }
    }

    /// Inserts a key-value parameter into the collection.
    pub fn insert(&mut self, key: impl Into<String>, value: impl Into<String>) {
        self.params.insert(key.into(), value.into());
    }

    /// Builder method to insert a key-value parameter.
    #[must_use]
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.insert(key, value);
        self
    }

    /// Looks up a parameter value by key.
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&str> {
        self.params.get(key).map(String::as_str)
    }

    /// Returns an iterator over all key-value parameter pairs.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.params.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Returns `true` if the collection contains a parameter with the specified key.
    #[must_use]
    pub fn contains(&self, key: &str) -> bool {
        self.params.contains_key(key)
    }

    /// Removes a parameter by key, returning its value if present.
    pub fn remove(&mut self, key: &str) -> Option<String> {
        self.params.remove(key)
    }

    /// Returns `true` if the collection contains no parameters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.params.is_empty()
    }

    /// Returns the number of parameters stored.
    #[must_use]
    pub fn len(&self) -> usize {
        self.params.len()
    }
}

/// A specific validation rule failure attributed to a hierarchical field path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationIssue {
    /// Hierarchical dot-and-bracket path (e.g. `user.addresses[0].postal_code`).
    pub field: String,
    /// Identifier of the rule that failed (e.g. `min_len`, `email`, `required`).
    pub rule: String,
    /// Human-readable localized error message.
    pub message: String,
    /// Parameter placeholders associated with the rule.
    pub params: RuleParams,
}

impl ValidationIssue {
    /// Creates a new validation issue.
    pub fn new(
        field: impl Into<String>,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            field: field.into(),
            rule: rule.into(),
            message: message.into(),
            params: RuleParams::new(),
        }
    }

    /// Attaches a single key-value parameter to the issue.
    #[must_use]
    pub fn with_param(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.params.insert(key, value);
        self
    }

    /// Replaces the rule parameters on this issue.
    #[must_use]
    pub fn with_params(mut self, params: RuleParams) -> Self {
        self.params = params;
        self
    }
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.field.is_empty() {
            write!(formatter, "{}: {}", self.rule, self.message)
        } else {
            write!(formatter, "{}: {}", self.field, self.message)
        }
    }
}

/// Collection of validation issues produced during schema evaluation.
///
/// Implements `std::error::Error` and provides formatting methods for both simple JSON error
/// arrays and official RFC 9457 Problem Details for HTTP APIs.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValidationError {
    issues: Vec<ValidationIssue>,
}

impl ValidationError {
    /// Creates a new empty `ValidationError` container.
    #[must_use]
    pub fn new() -> Self {
        Self { issues: Vec::new() }
    }

    /// Constructs a `ValidationError` containing a single issue.
    #[must_use]
    pub fn single(
        field: impl Into<String>,
        rule: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        let issue = ValidationIssue::new(field, rule, message);
        Self {
            issues: vec![issue],
        }
    }

    /// Adds a validation issue to the container.
    pub fn push(&mut self, issue: ValidationIssue) {
        self.issues.push(issue);
    }

    /// Builder method to add a validation issue.
    #[must_use]
    pub fn with_issue(mut self, issue: ValidationIssue) -> Self {
        self.push(issue);
        self
    }

    /// Merges another `ValidationError` container into this one.
    pub fn merge(&mut self, other: Self) {
        self.issues.extend(other.issues);
    }

    /// Prefixes all field paths with a given parent namespace.
    ///
    /// Useful for prefixing nested schemas or multi-source request components (e.g. `query.page`).
    pub fn prefix_paths(&mut self, prefix: &str) {
        if prefix.is_empty() {
            return;
        }
        for issue in &mut self.issues {
            if issue.field.is_empty() {
                issue.field = prefix.to_string();
            } else if issue.field.starts_with('[') {
                issue.field = format!("{prefix}{}", issue.field);
            } else {
                issue.field = format!("{prefix}.{}", issue.field);
            }
        }
    }

    /// Returns `true` if no validation issues were encountered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.issues.is_empty()
    }

    /// Returns the total count of validation issues.
    #[must_use]
    pub fn len(&self) -> usize {
        self.issues.len()
    }

    /// Constructs a `ValidationError` from an existing vector of issues.
    #[must_use]
    pub fn from_issues(issues: Vec<ValidationIssue>) -> Self {
        Self { issues }
    }

    /// Returns a reference to the first issue, if present.
    #[must_use]
    pub fn first_issue(&self) -> Option<&ValidationIssue> {
        self.issues.first()
    }

    /// Returns a slice of all validation issues.
    #[must_use]
    pub fn issues(&self) -> &[ValidationIssue] {
        &self.issues
    }

    /// Returns a mutable slice of all validation issues.
    pub fn issues_mut(&mut self) -> &mut [ValidationIssue] {
        &mut self.issues
    }

    /// Consumes the container and returns the inner vector of issues.
    #[must_use]
    pub fn into_issues(self) -> Vec<ValidationIssue> {
        self.issues
    }

    /// Formats the issues as a standard JSON error response payload.
    ///
    /// Produces:
    /// ```json
    /// {
    ///   "errors": [
    ///     {
    ///       "field": "user.email",
    ///       "rule": "email",
    ///       "message": "The email field must be a valid email address."
    ///     }
    ///   ]
    /// }
    /// ```
    #[must_use]
    pub fn to_json_value(&self) -> serde_json::Value {
        let error_items: Vec<serde_json::Value> = self
            .issues
            .iter()
            .map(|issue| {
                serde_json::json!({
                    "field": issue.field,
                    "rule": issue.rule,
                    "message": issue.message,
                })
            })
            .collect();

        serde_json::json!({
            "errors": error_items,
        })
    }

    /// Serializes the error report into a JSON formatted string.
    #[must_use]
    pub fn to_json_string(&self) -> String {
        serde_json::to_string(&self.to_json_value()).unwrap_or_else(|_| "{}".to_string())
    }

    /// Formats the issues as an RFC 9457 / RFC 7807 Problem Details object.
    ///
    /// Produces:
    /// ```json
    /// {
    ///   "type": "https://druto.dev/errors/validation",
    ///   "title": "Unprocessable Entity",
    ///   "status": 422,
    ///   "detail": "Validation failed for 1 field.",
    ///   "invalid_params": [
    ///     {
    ///       "name": "user.email",
    ///       "reason": "The email field must be a valid email address.",
    ///       "rule": "email"
    ///     }
    ///   ]
    /// }
    /// ```
    #[must_use]
    pub fn to_problem_details(&self, instance: Option<&str>) -> serde_json::Value {
        let invalid_params: Vec<serde_json::Value> = self
            .issues
            .iter()
            .map(|issue| {
                serde_json::json!({
                    "name": issue.field,
                    "reason": issue.message,
                    "rule": issue.rule,
                })
            })
            .collect();

        let detail_text = if self.issues.len() == 1 {
            "Validation failed for 1 field.".to_string()
        } else {
            format!("Validation failed for {} fields.", self.issues.len())
        };

        let mut doc = serde_json::json!({
            "type": "https://druto.dev/errors/validation",
            "title": "Unprocessable Entity",
            "status": 422,
            "detail": detail_text,
            "invalid_params": invalid_params,
        });

        if let (Some(uri), Some(map)) = (instance, doc.as_object_mut()) {
            map.insert(
                "instance".to_string(),
                serde_json::Value::String(uri.to_string()),
            );
        }

        doc
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.issues.is_empty() {
            write!(formatter, "Validation failed with 0 issues")
        } else if self.issues.len() == 1 {
            write!(formatter, "Validation error: {}", self.issues[0])
        } else {
            let details = self
                .issues
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join("; ");
            write!(
                formatter,
                "Validation failed with {} errors: [{details}]",
                self.issues.len()
            )
        }
    }
}

impl std::error::Error for ValidationError {}

impl<'a> IntoIterator for &'a ValidationError {
    type Item = &'a ValidationIssue;
    type IntoIter = std::slice::Iter<'a, ValidationIssue>;

    fn into_iter(self) -> Self::IntoIter {
        self.issues.iter()
    }
}
