//! String schema builder, string format rules, and sanitization.

use std::{
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
    sync::{Arc, LazyLock},
};

use regex::Regex;
use serde_json::Value;
#[cfg(feature = "async")]
use {crate::rule::AsyncCustomRule, futures_util::future::BoxFuture};

use crate::{
    coercion::trim_whitespace,
    context::ValidationContext,
    error::{RuleParams, ValidationError, ValidationIssue},
    messages::MessagesProvider,
    rule::{CustomRule, RuleContext},
    schema::Schema,
};

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)+$")
        .expect("Valid email regex")
});

static UUID_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[1-8][0-9a-fA-F]{3}-[89abAB][0-9a-fA-F]{3}-[0-9a-fA-F]{12}$")
        .expect("Valid UUID regex")
});

static URL_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^(?:https?|ftp)://[^\s/$.?#].[^\s]*$").expect("Valid URL regex"));

#[derive(Clone)]
enum StringPredicate {
    MinLen(usize),
    MaxLen(usize),
    ExactLen(usize),
    Email,
    Url,
    Uuid,
    Ip,
    Ipv4,
    Ipv6,
    RegexPattern(Regex),
    Alpha,
    Alphanumeric,
    Ascii,
    StartsWith(String),
    EndsWith(String),
    Contains(String),
    Custom(Arc<dyn CustomRule<String>>),
}

/// Fluent schema builder for validating and sanitizing string values.
#[derive(Clone)]
pub struct StringSchema {
    predicates: Vec<StringPredicate>,
    should_trim: bool,
    empty_as_null: bool,
    bail_on_fail: bool,
    inline_message: Option<String>,
    #[cfg(feature = "async")]
    async_rules: Vec<Arc<dyn AsyncCustomRule<String>>>,
}

impl Default for StringSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl StringSchema {
    /// Creates a new empty `StringSchema`.
    #[must_use]
    pub fn new() -> Self {
        Self {
            predicates: Vec::new(),
            should_trim: true,
            empty_as_null: false,
            bail_on_fail: false,
            inline_message: None,
            #[cfg(feature = "async")]
            async_rules: Vec::new(),
        }
    }

    /// Configures whether leading and trailing whitespace should be trimmed (default: `true`).
    #[must_use]
    pub fn trim(mut self, enabled: bool) -> Self {
        self.should_trim = enabled;
        self
    }

    /// Configures whether empty strings should be treated as null (default: `false`).
    #[must_use]
    pub fn empty_string_as_null(mut self, enabled: bool) -> Self {
        self.empty_as_null = enabled;
        self
    }

    /// Configures whether rule evaluation on this field should stop on the first failure.
    #[must_use]
    pub fn bail(mut self, enabled: bool) -> Self {
        self.bail_on_fail = enabled;
        self
    }

    /// Sets an inline custom error message override for this string field.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inline_message = Some(message.into());
        self
    }

    /// Enforces a minimum character length constraint.
    #[must_use]
    pub fn min_len(mut self, min: usize) -> Self {
        self.predicates.push(StringPredicate::MinLen(min));
        self
    }

    /// Enforces a maximum character length constraint.
    #[must_use]
    pub fn max_len(mut self, max: usize) -> Self {
        self.predicates.push(StringPredicate::MaxLen(max));
        self
    }

    /// Enforces an exact character length constraint.
    #[must_use]
    pub fn exact_len(mut self, length: usize) -> Self {
        self.predicates.push(StringPredicate::ExactLen(length));
        self
    }

    /// Validates that the string matches RFC 5322 email syntax.
    #[must_use]
    pub fn email(mut self) -> Self {
        self.predicates.push(StringPredicate::Email);
        self
    }

    /// Validates that the string matches RFC 3986 URL syntax.
    #[must_use]
    pub fn url(mut self) -> Self {
        self.predicates.push(StringPredicate::Url);
        self
    }

    /// Validates that the string is a valid UUID format (RFC 4122 / RFC 9562).
    #[must_use]
    pub fn uuid(mut self) -> Self {
        self.predicates.push(StringPredicate::Uuid);
        self
    }

    /// Validates that the string is either an IPv4 or IPv6 address.
    #[must_use]
    pub fn ip(mut self) -> Self {
        self.predicates.push(StringPredicate::Ip);
        self
    }

    /// Validates that the string is an IPv4 address.
    #[must_use]
    pub fn ipv4(mut self) -> Self {
        self.predicates.push(StringPredicate::Ipv4);
        self
    }

    /// Validates that the string is an IPv6 address.
    #[must_use]
    pub fn ipv6(mut self) -> Self {
        self.predicates.push(StringPredicate::Ipv6);
        self
    }

    /// Enforces matching against a custom regular expression string.
    #[must_use]
    pub fn regex(mut self, pattern: &str) -> Self {
        if let Ok(compiled) = Regex::new(pattern) {
            self.predicates
                .push(StringPredicate::RegexPattern(compiled));
        }
        self
    }

    /// Validates that the string contains only alphabetic characters.
    #[must_use]
    pub fn alpha(mut self) -> Self {
        self.predicates.push(StringPredicate::Alpha);
        self
    }

    /// Validates that the string contains only alphanumeric characters.
    #[must_use]
    pub fn alphanumeric(mut self) -> Self {
        self.predicates.push(StringPredicate::Alphanumeric);
        self
    }

    /// Validates that the string contains only ASCII characters.
    #[must_use]
    pub fn ascii(mut self) -> Self {
        self.predicates.push(StringPredicate::Ascii);
        self
    }

    /// Enforces that the string starts with the given prefix.
    #[must_use]
    pub fn starts_with(mut self, prefix: impl Into<String>) -> Self {
        self.predicates
            .push(StringPredicate::StartsWith(prefix.into()));
        self
    }

    /// Enforces that the string ends with the given suffix.
    #[must_use]
    pub fn ends_with(mut self, suffix: impl Into<String>) -> Self {
        self.predicates
            .push(StringPredicate::EndsWith(suffix.into()));
        self
    }

    /// Enforces that the string contains the specified substring.
    #[must_use]
    pub fn contains(mut self, expected: impl Into<String>) -> Self {
        self.predicates
            .push(StringPredicate::Contains(expected.into()));
        self
    }

    /// Attaches a custom synchronous validation rule.
    #[must_use]
    pub fn rule<R: CustomRule<String> + 'static>(mut self, rule: R) -> Self {
        self.predicates
            .push(StringPredicate::Custom(Arc::new(rule)));
        self
    }

    #[cfg(feature = "async")]
    /// Attaches an asynchronous custom validation rule.
    #[must_use]
    pub fn rule_async<R: AsyncCustomRule<String> + 'static>(mut self, rule: R) -> Self {
        self.async_rules.push(Arc::new(rule));
        self
    }

    #[cfg(feature = "async")]
    /// Refines the string value with an asynchronous predicate closure.
    #[must_use]
    pub fn refine_async<F>(mut self, predicate: F) -> Self
    where
        F: (for<'a> Fn(&'a str) -> BoxFuture<'a, Result<(), String>>) + Send + Sync + 'static,
    {
        let inline = self.inline_message.clone();
        let predicate = Arc::new(predicate);

        struct StringRefineRule<F> {
            predicate: Arc<F>,
            inline: Option<String>,
        }

        impl<F> AsyncCustomRule<String> for StringRefineRule<F>
        where
            F: (for<'a> Fn(&'a str) -> BoxFuture<'a, Result<(), String>>) + Send + Sync + 'static,
        {
            fn validate<'a>(
                &'a self,
                value: &'a String,
                ctx: &'a RuleContext<'a>,
            ) -> BoxFuture<'a, Result<(), ValidationIssue>> {
                Box::pin(async move {
                    match (self.predicate)(value.as_str()).await {
                        Ok(()) => Ok(()),
                        Err(err_msg) => {
                            let msg = self.inline.clone().unwrap_or(err_msg);
                            Err(ValidationIssue::new(ctx.path(), "refine_async", msg))
                        }
                    }
                })
            }
        }

        self.async_rules
            .push(Arc::new(StringRefineRule { predicate, inline }));
        self
    }

    fn evaluate_predicate(
        &self,
        predicate: &StringPredicate,
        value: &str,
        rule_context: &RuleContext<'_>,
    ) -> Option<ValidationIssue> {
        let inline = self.inline_message.as_deref();

        match predicate {
            StringPredicate::MinLen(min) => {
                let char_count = value.chars().count();
                if char_count < *min {
                    let mut params = RuleParams::new();
                    params.insert("min", min.to_string());
                    params.insert("actual", char_count.to_string());
                    Some(rule_context.create_issue("min_len", params, inline))
                } else {
                    None
                }
            }
            StringPredicate::MaxLen(max) => {
                let char_count = value.chars().count();
                if char_count > *max {
                    let mut params = RuleParams::new();
                    params.insert("max", max.to_string());
                    params.insert("actual", char_count.to_string());
                    Some(rule_context.create_issue("max_len", params, inline))
                } else {
                    None
                }
            }
            StringPredicate::ExactLen(expected) => {
                let char_count = value.chars().count();
                if char_count != *expected {
                    let mut params = RuleParams::new();
                    params.insert("length", expected.to_string());
                    params.insert("actual", char_count.to_string());
                    Some(rule_context.create_issue("exact_len", params, inline))
                } else {
                    None
                }
            }
            StringPredicate::Email => {
                if !EMAIL_REGEX.is_match(value) {
                    Some(rule_context.create_issue("email", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Url => {
                if !URL_REGEX.is_match(value) {
                    Some(rule_context.create_issue("url", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Uuid => {
                if !UUID_REGEX.is_match(value) {
                    Some(rule_context.create_issue("uuid", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Ip => {
                if IpAddr::from_str(value).is_err() {
                    Some(rule_context.create_issue("ip", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Ipv4 => {
                if Ipv4Addr::from_str(value).is_err() {
                    Some(rule_context.create_issue("ipv4", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Ipv6 => {
                if Ipv6Addr::from_str(value).is_err() {
                    Some(rule_context.create_issue("ipv6", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::RegexPattern(regex) => {
                if !regex.is_match(value) {
                    Some(rule_context.create_issue("regex", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Alpha => {
                let is_alpha = !value.is_empty() && value.chars().all(char::is_alphabetic);
                if !is_alpha {
                    Some(rule_context.create_issue("alpha", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Alphanumeric => {
                let is_alnum = !value.is_empty() && value.chars().all(char::is_alphanumeric);
                if !is_alnum {
                    Some(rule_context.create_issue("alphanumeric", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::Ascii => {
                if !value.is_ascii() {
                    Some(rule_context.create_issue("ascii", RuleParams::new(), inline))
                } else {
                    None
                }
            }
            StringPredicate::StartsWith(prefix) => {
                if !value.starts_with(prefix) {
                    let mut params = RuleParams::new();
                    params.insert("prefix", prefix);
                    Some(rule_context.create_issue("starts_with", params, inline))
                } else {
                    None
                }
            }
            StringPredicate::EndsWith(suffix) => {
                if !value.ends_with(suffix) {
                    let mut params = RuleParams::new();
                    params.insert("suffix", suffix);
                    Some(rule_context.create_issue("ends_with", params, inline))
                } else {
                    None
                }
            }
            StringPredicate::Contains(expected) => {
                if !value.contains(expected) {
                    let mut params = RuleParams::new();
                    params.insert("expected", expected);
                    Some(rule_context.create_issue("contains", params, inline))
                } else {
                    None
                }
            }
            StringPredicate::Custom(rule) => {
                let owned = value.to_string();
                rule.validate(&owned, rule_context).err()
            }
        }
    }
}

impl Schema<String> for StringSchema {
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<String, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);

        let initial_string = match value {
            Value::String(s) => s.clone(),
            Value::Null => {
                let issue = rule_ctx.create_issue(
                    "required",
                    RuleParams::new(),
                    self.inline_message.as_deref(),
                );
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
            _ => {
                let issue = rule_ctx.create_issue(
                    "string",
                    RuleParams::new(),
                    self.inline_message.as_deref(),
                );
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
        };

        let mut processed = initial_string;
        if self.should_trim {
            processed = trim_whitespace(&processed).to_string();
        }

        if self.empty_as_null && processed.is_empty() {
            let issue = rule_ctx.create_issue(
                "required",
                RuleParams::new(),
                self.inline_message.as_deref(),
            );
            return Err(ValidationError::single(
                issue.field,
                issue.rule,
                issue.message,
            ));
        }

        let mut errors = ValidationError::new();
        for predicate in &self.predicates {
            if let Some(issue) = self.evaluate_predicate(predicate, &processed, &rule_ctx) {
                errors.push(issue);
                if self.bail_on_fail {
                    break;
                }
            }
        }

        if errors.is_empty() {
            Ok(processed)
        } else {
            Err(errors)
        }
    }

    #[cfg(feature = "async")]
    fn validate_async_path<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<String, ValidationError>> {
        Box::pin(async move {
            let sync_result = self.validate_path(value, path, context, parent_data, messages);
            let valid_string = match sync_result {
                Ok(val) => val,
                Err(sync_err) => {
                    if self.bail_on_fail || self.async_rules.is_empty() {
                        return Err(sync_err);
                    }
                    return Err(sync_err);
                }
            };

            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let mut errors = ValidationError::new();

            for async_rule in &self.async_rules {
                match async_rule.validate(&valid_string, &rule_ctx).await {
                    Ok(()) => {}
                    Err(issue) => {
                        errors.push(issue);
                        if self.bail_on_fail {
                            break;
                        }
                    }
                }
            }

            if errors.is_empty() {
                Ok(valid_string)
            } else {
                Err(errors)
            }
        })
    }
}
