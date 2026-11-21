//! Object schema builder for validating structured dictionaries, cross-field rules, and fail-closed security.

use std::{collections::BTreeMap, sync::Arc};

#[cfg(feature = "async")]
use futures_util::future::BoxFuture;
use serde::de::DeserializeOwned;
use serde_json::{Map, Value};

use crate::{
    context::ValidationContext,
    error::{RuleParams, ValidationError, ValidationIssue},
    messages::{MessagesProvider, SimpleMessagesProvider},
    rule::RuleContext,
    schema::Schema,
};

/// Policy governing how object schemas handle undeclared/unexpected fields in incoming payloads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownFieldPolicy {
    /// Omit undeclared fields from the decoded output to prevent mass-assignment vulnerabilities.
    #[default]
    Strip,
    /// Reject payloads containing any undeclared field with an `unknown_field` validation error.
    Strict,
    /// Preserve all undeclared fields in the decoded output.
    Passthrough,
}

trait ErasedField: Send + Sync {
    fn is_optional(&self) -> bool;

    fn validate_field(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<Value, ValidationError>;

    #[cfg(feature = "async")]
    fn validate_async_field<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<Value, ValidationError>>;
}

struct FieldEntry<S, T> {
    schema: S,
    optional: bool,
    phantom: std::marker::PhantomData<T>,
}

impl<S, T> ErasedField for FieldEntry<S, T>
where
    S: Schema<T>,
    T: serde::Serialize + Send + Sync + 'static,
{
    fn is_optional(&self) -> bool {
        self.optional
    }

    fn validate_field(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<Value, ValidationError> {
        let typed_result =
            self.schema
                .validate_path(value, path, context, parent_data, messages)?;

        serde_json::to_value(&typed_result).map_err(|err| {
            let issue = ValidationIssue::new(path, "serialization", err.to_string());
            ValidationError::single(issue.field, issue.rule, issue.message)
        })
    }

    #[cfg(feature = "async")]
    fn validate_async_field<'a>(
        &'a self,
        value: &'a Value,
        path: &'a str,
        context: &'a ValidationContext,
        parent_data: Option<&'a Value>,
        messages: &'a dyn MessagesProvider,
    ) -> BoxFuture<'a, Result<Value, ValidationError>> {
        Box::pin(async move {
            let typed_result = self
                .schema
                .validate_async_path(value, path, context, parent_data, messages)
                .await?;

            serde_json::to_value(&typed_result).map_err(|err| {
                let issue = ValidationIssue::new(path, "serialization", err.to_string());
                ValidationError::single(issue.field, issue.rule, issue.message)
            })
        })
    }
}

#[derive(Clone)]
enum CrossFieldRule {
    SameAs {
        field: String,
        other: String,
        custom_message: Option<String>,
    },
    DifferentFrom {
        field: String,
        other: String,
        custom_message: Option<String>,
    },
    RequiredIf {
        field: String,
        other: String,
        expected: Value,
        custom_message: Option<String>,
    },
    RequiredWithout {
        field: String,
        other: String,
        custom_message: Option<String>,
    },
    AfterField {
        field: String,
        other: String,
        custom_message: Option<String>,
    },
    BeforeField {
        field: String,
        other: String,
        custom_message: Option<String>,
    },
}

/// Fluent schema builder for validating JSON objects, maps, and domain structures.
#[derive(Clone)]
pub struct ObjectSchema {
    fields: BTreeMap<String, Arc<dyn ErasedField>>,
    unknown_policy: UnknownFieldPolicy,
    cross_rules: Vec<CrossFieldRule>,
    custom_messages: SimpleMessagesProvider,
    bail_on_fail: bool,
    inline_message: Option<String>,
}

impl Default for ObjectSchema {
    fn default() -> Self {
        Self::new()
    }
}

impl ObjectSchema {
    /// Creates a new `ObjectSchema` defaulting to fail-closed `strip` policy.
    #[must_use]
    pub fn new() -> Self {
        Self {
            fields: BTreeMap::new(),
            unknown_policy: UnknownFieldPolicy::Strip,
            cross_rules: Vec::new(),
            custom_messages: SimpleMessagesProvider::default(),
            bail_on_fail: false,
            inline_message: None,
        }
    }

    /// Registers a required field schema.
    #[must_use]
    pub fn field<S, T>(mut self, name: impl Into<String>, schema: S) -> Self
    where
        S: Schema<T>,
        T: serde::Serialize + Send + Sync + 'static,
    {
        let entry = FieldEntry {
            schema,
            optional: false,
            phantom: std::marker::PhantomData,
        };
        self.fields.insert(name.into(), Arc::new(entry));
        self
    }

    /// Registers an optional field schema.
    #[must_use]
    pub fn optional_field<S, T>(mut self, name: impl Into<String>, schema: S) -> Self
    where
        S: Schema<T>,
        T: serde::Serialize + Send + Sync + 'static,
    {
        let entry = FieldEntry {
            schema,
            optional: true,
            phantom: std::marker::PhantomData,
        };
        self.fields.insert(name.into(), Arc::new(entry));
        self
    }

    /// Configures the schema to silently strip undeclared fields (default).
    #[must_use]
    pub fn strip(mut self) -> Self {
        self.unknown_policy = UnknownFieldPolicy::Strip;
        self
    }

    /// Configures the schema to strictly reject undeclared fields with `422 Unprocessable Entity`.
    #[must_use]
    pub fn strict(mut self) -> Self {
        self.unknown_policy = UnknownFieldPolicy::Strict;
        self
    }

    /// Configures the schema to preserve undeclared fields in the decoded output.
    #[must_use]
    pub fn passthrough(mut self) -> Self {
        self.unknown_policy = UnknownFieldPolicy::Passthrough;
        self
    }

    /// Configures short-circuiting on failure across fields.
    #[must_use]
    pub fn bail(mut self, enabled: bool) -> Self {
        self.bail_on_fail = enabled;
        self
    }

    /// Sets an inline custom error message override.
    #[must_use]
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.inline_message = Some(message.into());
        self
    }

    /// Overrides error messages for specific fields or rules.
    #[must_use]
    pub fn messages(mut self, overrides: &[(&str, &str)]) -> Self {
        for (key, template) in overrides {
            if let Some((field, rule)) = key.split_once('.') {
                if rule == "*" {
                    self.custom_messages.set_field_wildcard(field, *template);
                } else {
                    self.custom_messages.set_field_rule(field, rule, *template);
                }
            } else {
                self.custom_messages.set_rule_default(*key, *template);
            }
        }
        self
    }

    /// Asserts that `field` and `other` contain identical values (e.g. password confirmation).
    #[must_use]
    pub fn same_as(mut self, field: impl Into<String>, other: impl Into<String>) -> Self {
        self.cross_rules.push(CrossFieldRule::SameAs {
            field: field.into(),
            other: other.into(),
            custom_message: None,
        });
        self
    }

    /// Asserts that `field` and `other` contain different values.
    #[must_use]
    pub fn different_from(mut self, field: impl Into<String>, other: impl Into<String>) -> Self {
        self.cross_rules.push(CrossFieldRule::DifferentFrom {
            field: field.into(),
            other: other.into(),
            custom_message: None,
        });
        self
    }

    /// Asserts that `field` is present and non-null when `other` matches `expected`.
    #[must_use]
    pub fn required_if(
        mut self,
        field: impl Into<String>,
        other: impl Into<String>,
        expected: impl Into<Value>,
    ) -> Self {
        self.cross_rules.push(CrossFieldRule::RequiredIf {
            field: field.into(),
            other: other.into(),
            expected: expected.into(),
            custom_message: None,
        });
        self
    }

    /// Asserts that `field` is present and non-null when `other` is absent or null.
    #[must_use]
    pub fn required_without(mut self, field: impl Into<String>, other: impl Into<String>) -> Self {
        self.cross_rules.push(CrossFieldRule::RequiredWithout {
            field: field.into(),
            other: other.into(),
            custom_message: None,
        });
        self
    }

    /// Asserts that `field` is temporally or numerically after `other`.
    #[must_use]
    pub fn after_field(mut self, field: impl Into<String>, other: impl Into<String>) -> Self {
        self.cross_rules.push(CrossFieldRule::AfterField {
            field: field.into(),
            other: other.into(),
            custom_message: None,
        });
        self
    }

    /// Asserts that `field` is temporally or numerically before `other`.
    #[must_use]
    pub fn before_field(mut self, field: impl Into<String>, other: impl Into<String>) -> Self {
        self.cross_rules.push(CrossFieldRule::BeforeField {
            field: field.into(),
            other: other.into(),
            custom_message: None,
        });
        self
    }

    /// Validates the payload and decodes directly into a typed struct `T: DeserializeOwned`.
    pub fn decode<T: DeserializeOwned>(&self, value: &Value) -> Result<T, ValidationError> {
        let validated_json = self.validate(value)?;
        serde_json::from_value(validated_json).map_err(|err| {
            let issue = ValidationIssue::new("", "deserialization", err.to_string());
            ValidationError::single(issue.field, issue.rule, issue.message)
        })
    }

    /// Converts this `ObjectSchema` into a `Schema<T>` that automatically decodes into `T: DeserializeOwned`.
    pub fn decode_into<T>(
        self,
    ) -> crate::schema::TransformSchema<
        Self,
        Value,
        T,
        impl Fn(Value) -> Result<T, String> + Send + Sync + 'static,
    >
    where
        T: DeserializeOwned + Send + Sync + 'static,
    {
        self.transform(|val| serde_json::from_value(val).map_err(|err| err.to_string()))
    }

    #[cfg(feature = "async")]
    /// Asynchronously validates the payload and decodes directly into `T: DeserializeOwned`.
    pub async fn decode_async<T: DeserializeOwned>(
        &self,
        value: &Value,
    ) -> Result<T, ValidationError> {
        let validated_json = self.validate_async(value).await?;
        serde_json::from_value(validated_json).map_err(|err| {
            let issue = ValidationIssue::new("", "deserialization", err.to_string());
            ValidationError::single(issue.field, issue.rule, issue.message)
        })
    }

    fn evaluate_cross_rules(
        &self,
        map: &Map<String, Value>,
        path: &str,
        errors: &mut ValidationError,
    ) {
        for cross_rule in &self.cross_rules {
            match cross_rule {
                CrossFieldRule::SameAs {
                    field,
                    other,
                    custom_message,
                } => {
                    let val_a = map.get(field).unwrap_or(&Value::Null);
                    let val_b = map.get(other).unwrap_or(&Value::Null);

                    if val_a != val_b {
                        let full_path = qualify_path(path, field);
                        let mut params = RuleParams::new();
                        params.insert("field", field.clone());
                        params.insert("other", other.clone());
                        let msg = custom_message
                            .as_deref()
                            .unwrap_or("The :field and :other fields must match.");
                        let rendered = crate::messages::interpolate_placeholders(
                            msg, field, "same_as", &params,
                        );
                        errors.push(ValidationIssue {
                            field: full_path,
                            rule: "same_as".to_string(),
                            message: rendered,
                            params,
                        });
                        if self.bail_on_fail {
                            return;
                        }
                    }
                }
                CrossFieldRule::DifferentFrom {
                    field,
                    other,
                    custom_message,
                } => {
                    let val_a = map.get(field).unwrap_or(&Value::Null);
                    let val_b = map.get(other).unwrap_or(&Value::Null);

                    if !val_a.is_null() && !val_b.is_null() && val_a == val_b {
                        let full_path = qualify_path(path, field);
                        let mut params = RuleParams::new();
                        params.insert("field", field.clone());
                        params.insert("other", other.clone());
                        let msg = custom_message
                            .as_deref()
                            .unwrap_or("The :field and :other fields must be different.");
                        let rendered = crate::messages::interpolate_placeholders(
                            msg,
                            field,
                            "different_from",
                            &params,
                        );
                        errors.push(ValidationIssue {
                            field: full_path,
                            rule: "different_from".to_string(),
                            message: rendered,
                            params,
                        });
                        if self.bail_on_fail {
                            return;
                        }
                    }
                }
                CrossFieldRule::RequiredIf {
                    field,
                    other,
                    expected,
                    custom_message,
                } => {
                    let dependency = map.get(other).unwrap_or(&Value::Null);
                    let matches_dependency = dependency == expected;

                    if matches_dependency {
                        let target = map.get(field).unwrap_or(&Value::Null);
                        let is_missing = target.is_null()
                            || (target.is_string()
                                && target.as_str().unwrap_or("").trim().is_empty());
                        if is_missing {
                            let full_path = qualify_path(path, field);
                            let mut params = RuleParams::new();
                            params.insert("field", field.clone());
                            params.insert("other", other.clone());
                            params.insert("value", expected.to_string());
                            let msg = custom_message
                                .as_deref()
                                .unwrap_or("The :field field is required when :other is :value.");
                            let rendered = crate::messages::interpolate_placeholders(
                                msg,
                                field,
                                "required_if",
                                &params,
                            );
                            errors.push(ValidationIssue {
                                field: full_path,
                                rule: "required_if".to_string(),
                                message: rendered,
                                params,
                            });
                            if self.bail_on_fail {
                                return;
                            }
                        }
                    }
                }
                CrossFieldRule::RequiredWithout {
                    field,
                    other,
                    custom_message,
                } => {
                    let dependency = map.get(other).unwrap_or(&Value::Null);
                    let dependency_is_absent = dependency.is_null()
                        || (dependency.is_string()
                            && dependency.as_str().unwrap_or("").trim().is_empty());

                    if dependency_is_absent {
                        let target = map.get(field).unwrap_or(&Value::Null);
                        let is_missing = target.is_null()
                            || (target.is_string()
                                && target.as_str().unwrap_or("").trim().is_empty());
                        if is_missing {
                            let full_path = qualify_path(path, field);
                            let mut params = RuleParams::new();
                            params.insert("field", field.clone());
                            params.insert("other", other.clone());
                            let msg = custom_message
                                .as_deref()
                                .unwrap_or("The :field field is required when :other is absent.");
                            let rendered = crate::messages::interpolate_placeholders(
                                msg,
                                field,
                                "required_without",
                                &params,
                            );
                            errors.push(ValidationIssue {
                                field: full_path,
                                rule: "required_without".to_string(),
                                message: rendered,
                                params,
                            });
                            if self.bail_on_fail {
                                return;
                            }
                        }
                    }
                }
                CrossFieldRule::AfterField {
                    field,
                    other,
                    custom_message,
                } => {
                    let val_a = map.get(field).unwrap_or(&Value::Null);
                    let val_b = map.get(other).unwrap_or(&Value::Null);

                    if let (Some(sa), Some(sb)) = (val_a.as_str(), val_b.as_str())
                        && sa <= sb
                    {
                        let full_path = qualify_path(path, field);
                        let mut params = RuleParams::new();
                        params.insert("field", field.clone());
                        params.insert("target", other.clone());
                        let msg = custom_message
                            .as_deref()
                            .unwrap_or("The :field field must be a date after :target.");
                        let rendered =
                            crate::messages::interpolate_placeholders(msg, field, "after", &params);
                        errors.push(ValidationIssue {
                            field: full_path,
                            rule: "after".to_string(),
                            message: rendered,
                            params,
                        });
                        if self.bail_on_fail {
                            return;
                        }
                    } else if let (Some(na), Some(nb)) = (val_a.as_f64(), val_b.as_f64())
                        && na <= nb
                    {
                        let full_path = qualify_path(path, field);
                        let mut params = RuleParams::new();
                        params.insert("field", field.clone());
                        params.insert("target", other.clone());
                        let msg = custom_message
                            .as_deref()
                            .unwrap_or("The :field field must be greater than :target.");
                        let rendered =
                            crate::messages::interpolate_placeholders(msg, field, "after", &params);
                        errors.push(ValidationIssue {
                            field: full_path,
                            rule: "after".to_string(),
                            message: rendered,
                            params,
                        });
                        if self.bail_on_fail {
                            return;
                        }
                    }
                }
                CrossFieldRule::BeforeField {
                    field,
                    other,
                    custom_message,
                } => {
                    let val_a = map.get(field).unwrap_or(&Value::Null);
                    let val_b = map.get(other).unwrap_or(&Value::Null);

                    if let (Some(sa), Some(sb)) = (val_a.as_str(), val_b.as_str())
                        && sa >= sb
                    {
                        let full_path = qualify_path(path, field);
                        let mut params = RuleParams::new();
                        params.insert("field", field.clone());
                        params.insert("target", other.clone());
                        let msg = custom_message
                            .as_deref()
                            .unwrap_or("The :field field must be a date before :target.");
                        let rendered = crate::messages::interpolate_placeholders(
                            msg, field, "before", &params,
                        );
                        errors.push(ValidationIssue {
                            field: full_path,
                            rule: "before".to_string(),
                            message: rendered,
                            params,
                        });
                        if self.bail_on_fail {
                            return;
                        }
                    } else if let (Some(na), Some(nb)) = (val_a.as_f64(), val_b.as_f64())
                        && na >= nb
                    {
                        let full_path = qualify_path(path, field);
                        let mut params = RuleParams::new();
                        params.insert("field", field.clone());
                        params.insert("target", other.clone());
                        let msg = custom_message
                            .as_deref()
                            .unwrap_or("The :field field must be less than :target.");
                        let rendered = crate::messages::interpolate_placeholders(
                            msg, field, "before", &params,
                        );
                        errors.push(ValidationIssue {
                            field: full_path,
                            rule: "before".to_string(),
                            message: rendered,
                            params,
                        });
                        if self.bail_on_fail {
                            return;
                        }
                    }
                }
            }
        }
    }
}

impl Schema<Value> for ObjectSchema {
    fn validate_path(
        &self,
        value: &Value,
        path: &str,
        context: &ValidationContext,
        parent_data: Option<&Value>,
        messages: &dyn MessagesProvider,
    ) -> Result<Value, ValidationError> {
        let rule_ctx = RuleContext::new(path, context, parent_data, messages);
        let inline = self.inline_message.as_deref();

        let map = match value {
            Value::Object(obj) => obj,
            Value::Null => {
                let issue = rule_ctx.create_issue("required", RuleParams::new(), inline);
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
            _ => {
                let issue = rule_ctx.create_issue("required", RuleParams::new(), inline);
                return Err(ValidationError::single(
                    issue.field,
                    issue.rule,
                    issue.message,
                ));
            }
        };

        let mut errors = ValidationError::new();
        let mut result_map = Map::new();

        // 1. Check for unexpected unknown fields
        if self.unknown_policy == UnknownFieldPolicy::Strict {
            for key in map.keys() {
                if !self.fields.contains_key(key) {
                    let field_path = qualify_path(path, key);
                    let mut params = RuleParams::new();
                    params.insert("field", key.clone());
                    let issue = rule_ctx.create_issue("unknown_field", params, inline);
                    errors.push(ValidationIssue {
                        field: field_path,
                        rule: "unknown_field".to_string(),
                        message: issue.message,
                        params: issue.params,
                    });
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                }
            }
        }

        // 2. Validate declared fields
        for (field_name, field_def) in &self.fields {
            let field_path = qualify_path(path, field_name);
            let input_val = map.get(field_name).unwrap_or(&Value::Null);

            if input_val.is_null() && field_def.is_optional() {
                continue;
            }

            match field_def.validate_field(
                input_val,
                &field_path,
                context,
                Some(value),
                &self.custom_messages,
            ) {
                Ok(valid_val) => {
                    result_map.insert(field_name.clone(), valid_val);
                }
                Err(field_err) => {
                    errors.merge(field_err);
                    if self.bail_on_fail {
                        return Err(errors);
                    }
                }
            }
        }

        // 3. Passthrough extra fields if policy is Passthrough
        if self.unknown_policy == UnknownFieldPolicy::Passthrough {
            for (key, val) in map {
                if !self.fields.contains_key(key) {
                    result_map.insert(key.clone(), val.clone());
                }
            }
        }

        // 4. Cross-field validation rules
        self.evaluate_cross_rules(map, path, &mut errors);

        if errors.is_empty() {
            Ok(Value::Object(result_map))
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
    ) -> BoxFuture<'a, Result<Value, ValidationError>> {
        Box::pin(async move {
            let rule_ctx = RuleContext::new(path, context, parent_data, messages);
            let inline = self.inline_message.as_deref();

            let map = match value {
                Value::Object(obj) => obj,
                Value::Null => {
                    let issue = rule_ctx.create_issue("required", RuleParams::new(), inline);
                    return Err(ValidationError::single(
                        issue.field,
                        issue.rule,
                        issue.message,
                    ));
                }
                _ => {
                    let issue = rule_ctx.create_issue("required", RuleParams::new(), inline);
                    return Err(ValidationError::single(
                        issue.field,
                        issue.rule,
                        issue.message,
                    ));
                }
            };

            let mut errors = ValidationError::new();
            let mut result_map = Map::new();

            if self.unknown_policy == UnknownFieldPolicy::Strict {
                for key in map.keys() {
                    if !self.fields.contains_key(key) {
                        let field_path = qualify_path(path, key);
                        let mut params = RuleParams::new();
                        params.insert("field", key.clone());
                        let issue = rule_ctx.create_issue("unknown_field", params, inline);
                        errors.push(ValidationIssue {
                            field: field_path,
                            rule: "unknown_field".to_string(),
                            message: issue.message,
                            params: issue.params,
                        });
                        if self.bail_on_fail {
                            return Err(errors);
                        }
                    }
                }
            }

            for (field_name, field_def) in &self.fields {
                let field_path = qualify_path(path, field_name);
                let input_val = map.get(field_name).unwrap_or(&Value::Null);

                if input_val.is_null() && field_def.is_optional() {
                    continue;
                }

                match field_def
                    .validate_async_field(
                        input_val,
                        &field_path,
                        context,
                        Some(value),
                        &self.custom_messages,
                    )
                    .await
                {
                    Ok(valid_val) => {
                        result_map.insert(field_name.clone(), valid_val);
                    }
                    Err(field_err) => {
                        errors.merge(field_err);
                        if self.bail_on_fail {
                            return Err(errors);
                        }
                    }
                }
            }

            if self.unknown_policy == UnknownFieldPolicy::Passthrough {
                for (key, val) in map {
                    if !self.fields.contains_key(key) {
                        result_map.insert(key.clone(), val.clone());
                    }
                }
            }

            self.evaluate_cross_rules(map, path, &mut errors);

            if errors.is_empty() {
                Ok(Value::Object(result_map))
            } else {
                Err(errors)
            }
        })
    }
}

fn qualify_path(base_path: &str, field_name: &str) -> String {
    if base_path.is_empty() {
        field_name.to_string()
    } else {
        format!("{base_path}.{field_name}")
    }
}
