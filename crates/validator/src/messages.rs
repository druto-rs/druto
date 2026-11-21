//! Localized error message templates, placeholder interpolation, and cascading message providers.

use std::{collections::HashMap, sync::Arc};

use crate::error::RuleParams;

/// Formats and resolves error message copy for validation failures.
pub trait MessagesProvider: Send + Sync {
    /// Resolves and formats a message string for a given field, rule, and parameters.
    fn format_message(
        &self,
        field: &str,
        rule: &str,
        params: &RuleParams,
        inline_override: Option<&str>,
    ) -> String;
}

/// In-memory message provider supporting cascading resolution and placeholder interpolation.
///
/// Resolution priority:
/// 1. Inline rule message (`.with_message(...)`)
/// 2. Field-specific rule override (`"email.email"`)
/// 3. Field wildcard override (`"email.*"`)
/// 4. Rule-level override (`"email"`)
/// 5. Built-in standard template
/// 6. Generic fallback message
#[derive(Debug, Clone)]
pub struct SimpleMessagesProvider {
    rule_defaults: HashMap<String, String>,
    field_overrides: HashMap<String, String>,
}

impl Default for SimpleMessagesProvider {
    fn default() -> Self {
        let mut provider = Self {
            rule_defaults: HashMap::new(),
            field_overrides: HashMap::new(),
        };
        provider.init_built_in_defaults();
        provider
    }
}

impl SimpleMessagesProvider {
    /// Creates a new `SimpleMessagesProvider` initialized with built-in default templates.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets a default message template for a specific rule identifier.
    pub fn set_rule_default(&mut self, rule: impl Into<String>, template: impl Into<String>) {
        self.rule_defaults.insert(rule.into(), template.into());
    }

    /// Sets an override message template for a specific field and rule combination.
    ///
    /// For example: `provider.set_field_rule("user.email", "email", "Please provide a valid corporate email.")`
    pub fn set_field_rule(
        &mut self,
        field: impl Into<String>,
        rule: impl Into<String>,
        template: impl Into<String>,
    ) {
        let key = format!("{}.{}", field.into(), rule.into());
        self.field_overrides.insert(key, template.into());
    }

    /// Sets an override message template for any failure on a specific field.
    ///
    /// For example: `provider.set_field_wildcard("user.email", "Invalid email address.")`
    pub fn set_field_wildcard(&mut self, field: impl Into<String>, template: impl Into<String>) {
        let key = format!("{}.*", field.into());
        self.field_overrides.insert(key, template.into());
    }

    fn init_built_in_defaults(&mut self) {
        let defaults = [
            ("required", "The :field field is required."),
            ("email", "The :field field must be a valid email address."),
            (
                "min_len",
                "The :field field must be at least :min characters.",
            ),
            (
                "max_len",
                "The :field field may not be greater than :max characters.",
            ),
            (
                "exact_len",
                "The :field field must be exactly :length characters.",
            ),
            ("url", "The :field field must be a valid URL."),
            ("uuid", "The :field field must be a valid UUID."),
            ("ip", "The :field field must be a valid IP address."),
            ("ipv4", "The :field field must be a valid IPv4 address."),
            ("ipv6", "The :field field must be a valid IPv6 address."),
            ("regex", "The :field field format is invalid."),
            ("alpha", "The :field field must only contain letters."),
            (
                "alphanumeric",
                "The :field field must only contain letters and numbers.",
            ),
            (
                "ascii",
                "The :field field must contain only ASCII characters.",
            ),
            ("starts_with", "The :field field must start with :prefix."),
            ("ends_with", "The :field field must end with :suffix."),
            ("contains", "The :field field must contain :expected."),
            ("min", "The :field field must be at least :min."),
            ("max", "The :field field must not exceed :max."),
            ("range", "The :field field must be between :min and :max."),
            ("integer", "The :field field must be an integer."),
            ("positive", "The :field field must be a positive number."),
            ("negative", "The :field field must be a negative number."),
            (
                "multiple_of",
                "The :field field must be a multiple of :step.",
            ),
            ("accepted", "The :field field must be accepted."),
            ("date", "The :field field must be a valid ISO 8601 date."),
            ("before", "The :field field must be a date before :target."),
            ("after", "The :field field must be a date after :target."),
            ("same_as", "The :field and :other fields must match."),
            (
                "different_from",
                "The :field and :other fields must be different.",
            ),
            (
                "required_if",
                "The :field field is required when :other is :value.",
            ),
            (
                "required_without",
                "The :field field is required when :other is absent.",
            ),
            (
                "min_items",
                "The :field field must have at least :min items.",
            ),
            (
                "max_items",
                "The :field field must not have more than :max items.",
            ),
            (
                "exact_items",
                "The :field field must contain exactly :length items.",
            ),
            ("not_empty", "The :field field cannot be empty."),
            ("distinct", "The :field field has duplicate elements."),
            (
                "enum",
                "The selected :field is invalid. Expected one of: :expected.",
            ),
            ("unknown_field", "Field ':field' is not recognized."),
        ];

        for (rule, template) in defaults {
            self.rule_defaults
                .insert(rule.to_string(), template.to_string());
        }
    }
}

impl MessagesProvider for SimpleMessagesProvider {
    fn format_message(
        &self,
        field: &str,
        rule: &str,
        params: &RuleParams,
        inline_override: Option<&str>,
    ) -> String {
        let field_rule_key = format!("{field}.{rule}");
        let field_wildcard_key = format!("{field}.*");

        let raw_template = inline_override
            .or_else(|| {
                self.field_overrides
                    .get(&field_rule_key)
                    .map(String::as_str)
            })
            .or_else(|| {
                self.field_overrides
                    .get(&field_wildcard_key)
                    .map(String::as_str)
            })
            .or_else(|| self.rule_defaults.get(rule).map(String::as_str))
            .unwrap_or("The :field field failed validation (:rule).");

        interpolate_placeholders(raw_template, field, rule, params)
    }
}

/// Interpolates `:field`, `:rule`, and custom parameter placeholders into a message template.
pub fn interpolate_placeholders(
    template: &str,
    field: &str,
    rule: &str,
    params: &RuleParams,
) -> String {
    let mut rendered = template.to_string();

    let display_field = humanize_field_name(field);
    rendered = rendered.replace(":field", &display_field);
    rendered = rendered.replace(":rule", rule);

    for (key, val) in params.iter() {
        let placeholder = format!(":{key}");
        rendered = rendered.replace(&placeholder, val);
    }

    rendered
}

/// Derives a clean human-readable name from a hierarchical field path.
///
/// For example: `user.addresses[0].postal_code` -> `postal code`
#[must_use]
pub fn humanize_field_name(field: &str) -> String {
    if field.is_empty() {
        return "value".to_string();
    }

    let segment = field.rsplit('.').next().unwrap_or(field);

    let clean = if let Some(bracket_idx) = segment.rfind('[') {
        &segment[bracket_idx + 1..segment.len().saturating_sub(1)]
    } else {
        segment
    };

    clean.replace('_', " ")
}

/// Shared reference to a thread-safe message provider.
pub type SharedMessagesProvider = Arc<dyn MessagesProvider>;
