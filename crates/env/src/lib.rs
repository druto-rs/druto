//! Environment variable loading, cascading, expansion, and schema validation.
//!
//! # Architecture & Features
//! - **Cascading File Discovery**: Supports `.env.{env}.local` -> `.env.local` -> `.env.{env}` -> `.env`.
//! - **Variable Interpolation**: Expands `${VAR}`, `$VAR`, and fallbacks `${VAR:-default}` with cycle detection.
//! - **Container Secrets**: Built-in `file:<path>` prefix resolver for Docker and Kubernetes secrets.
//! - **Schema Validation**: Fluent rules for `string`, `integer`, `float`, `boolean`, `port`, `url`, `duration`, `bytes`, and `enum_values`.
//! - **Aggregated Diagnostics**: Reports all configuration failures in a single pass without leaking secrets.
//! - **Secret Protection**: Dedicated `Secret<T>` wrapper masking values as `[REDACTED]`.
//! - **Dotenv Editor**: Programmatic editing of `.env` files preserving comments and layout.

pub mod editor;
pub mod env;
pub mod error;
pub mod interpolator;
pub mod loader;
pub mod parser;
pub mod rules;
pub mod secret;
pub mod validator;

pub use editor::{EditorLine, EnvEditor};
pub use env::Env;
pub use error::{EnvError, ValidationError};
pub use interpolator::EnvInterpolator;
pub use loader::EnvLoader;
pub use parser::{EnvParser, ParsedEntry};
pub use rules::{FieldValidator, Rule};
pub use secret::Secret;
pub use validator::EnvValidator;

impl Env {
    /// Creates a new fluent `EnvValidator` schema builder.
    pub fn rules() -> EnvValidator {
        EnvValidator::new()
    }

    /// Creates a new `EnvLoader` for discovering and reading `.env` files.
    pub fn loader() -> EnvLoader {
        EnvLoader::new()
    }
}
