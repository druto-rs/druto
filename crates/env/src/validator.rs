use std::{any::Any, collections::HashMap};

use crate::{
    env::Env,
    error::EnvError,
    loader::EnvLoader,
    rules::{FieldValidator, Rule},
};

/// Collects schema validation rules and executes comprehensive validation against environment variables.
#[derive(Default)]
pub struct EnvValidator {
    rules: Vec<(String, Box<dyn FieldValidator>)>,
}

impl EnvValidator {
    /// Creates a new empty `EnvValidator`.
    pub fn new() -> Self {
        Self { rules: Vec::new() }
    }

    /// Registers a validation rule for the specified environment variable name.
    pub fn rule<T: Send + Sync + 'static>(mut self, key: impl Into<String>, rule: Rule<T>) -> Self {
        self.rules.push((key.into(), Box::new(rule)));
        self
    }

    /// Validates the provided raw key-value mapping against all registered rules.
    ///
    /// Unlike fail-fast validators, `EnvValidator` aggregates all missing or invalid variables
    /// into a single `EnvError::ValidationFailed` error report so configuration problems can be
    /// resolved all at once.
    pub fn validate(&self, raw_map: &HashMap<String, String>) -> Result<Env, EnvError> {
        let mut errors = Vec::new();
        let mut typed_values: HashMap<String, Box<dyn Any + Send + Sync>> =
            HashMap::with_capacity(self.rules.len());

        for (key, validator) in &self.rules {
            match validator.validate(key, raw_map) {
                Ok(boxed_value) => {
                    typed_values.insert(key.clone(), boxed_value);
                }
                Err(validation_error) => {
                    errors.push(validation_error);
                }
            }
        }

        if !errors.is_empty() {
            Err(EnvError::ValidationFailed { errors })
        } else {
            Ok(Env::new(raw_map.clone(), typed_values))
        }
    }

    /// Convenience method that runs `EnvLoader::load` and then executes schema validation.
    pub fn load_and_validate(&self, loader: &EnvLoader) -> Result<Env, EnvError> {
        let raw_map = loader.load()?;
        self.validate(&raw_map)
    }

    /// Convenience method that parses raw string contents and then executes schema validation.
    pub fn parse_and_validate(&self, contents: &str) -> Result<Env, EnvError> {
        let raw_map = EnvLoader::new().load_from_str(contents)?;
        self.validate(&raw_map)
    }
}
