use std::{error::Error, fmt, io};

/// A single validation failure for an environment variable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationError {
    /// The environment variable name that failed validation.
    pub key: String,
    /// Human-readable description of why validation failed.
    pub message: String,
}

impl ValidationError {
    /// Creates a new validation error.
    pub fn new(key: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}: {}", self.key, self.message)
    }
}

/// Errors that can occur during environment variable loading, parsing, interpolation, or validation.
#[derive(Debug)]
pub enum EnvError {
    /// Underlying file or I/O error.
    Io(io::Error),
    /// Syntax error when parsing a `.env` file.
    Parse {
        /// Line number where the syntax error occurred.
        line: usize,
        /// Description of the parsing error.
        message: String,
    },
    /// Cyclic dependency detected during variable expansion.
    CyclicDependency {
        /// The resolution stack forming the cycle.
        cycle: Vec<String>,
    },
    /// Prefix resolver failure (e.g. `file:<path>` file missing).
    Resolver {
        /// The key of the variable whose resolver failed.
        key: String,
        /// Resolver failure details.
        message: String,
    },
    /// One or more environment variables failed schema validation.
    ValidationFailed {
        /// All aggregated validation errors.
        errors: Vec<ValidationError>,
    },
    /// Requested variable was not found in the environment.
    NotFound {
        /// Missing variable key.
        key: String,
    },
    /// Variable value could not be converted to the requested type.
    TypeMismatch {
        /// Variable key with mismatched type.
        key: String,
        /// Expected type name.
        expected: &'static str,
    },
    /// Deserialization error when mapping to a struct.
    #[cfg(feature = "serde")]
    Deserialize(String),
}

impl fmt::Display for EnvError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(err) => write!(formatter, "I/O error: {err}"),
            Self::Parse { line, message } => {
                write!(formatter, "parse error at line {line}: {message}")
            }
            Self::CyclicDependency { cycle } => {
                write!(
                    formatter,
                    "cyclic dependency detected in variable interpolation: {}",
                    cycle.join(" -> ")
                )
            }
            Self::Resolver { key, message } => {
                write!(formatter, "resolver failed for `{key}`: {message}")
            }
            Self::ValidationFailed { errors } => {
                writeln!(
                    formatter,
                    "environment validation failed with {} error(s):",
                    errors.len()
                )?;
                for error in errors {
                    writeln!(formatter, "  - {}: {}", error.key, error.message)?;
                }
                Ok(())
            }
            Self::NotFound { key } => {
                write!(formatter, "environment variable `{key}` not found")
            }
            Self::TypeMismatch { key, expected } => {
                write!(formatter, "type mismatch for `{key}`: expected {expected}")
            }
            #[cfg(feature = "serde")]
            Self::Deserialize(message) => {
                write!(formatter, "deserialization error: {message}")
            }
        }
    }
}

impl Error for EnvError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for EnvError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
