use std::{collections::HashMap, fs, path::PathBuf};

use crate::{error::EnvError, interpolator::EnvInterpolator, parser::EnvParser};

/// Configures and executes cascading `.env` file discovery, loading, and merging.
#[derive(Debug, Clone)]
pub struct EnvLoader {
    base_dir: PathBuf,
    active_env: Option<String>,
    override_system: bool,
    explicit_file: Option<PathBuf>,
}

impl Default for EnvLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl EnvLoader {
    /// Creates a new `EnvLoader` pointing to the current working directory.
    pub fn new() -> Self {
        Self {
            base_dir: PathBuf::from("."),
            active_env: None,
            override_system: false,
            explicit_file: None,
        }
    }

    /// Sets the base directory from which `.env` files are loaded.
    pub fn with_base_dir(mut self, path: impl Into<PathBuf>) -> Self {
        self.base_dir = path.into();
        self
    }

    /// Sets the active environment name (e.g. `"development"`, `"production"`, `"test"`).
    pub fn with_env(mut self, env_name: impl Into<String>) -> Self {
        self.active_env = Some(env_name.into());
        self
    }

    /// Configures whether `.env` file values should override existing OS environment variables.
    pub fn override_system(mut self, override_system: bool) -> Self {
        self.override_system = override_system;
        self
    }

    /// Directs the loader to read an explicit `.env` file instead of cascading.
    pub fn with_file(mut self, file_path: impl Into<PathBuf>) -> Self {
        self.explicit_file = Some(file_path.into());
        self
    }

    /// Resolves the active environment name from explicit config, `DRUTO_ENV`, `APP_ENV`, or defaults to `"development"`.
    pub fn resolve_active_env(&self) -> String {
        if let Some(env_name) = &self.active_env {
            return env_name.clone();
        }

        if let Ok(env_name) = std::env::var("DRUTO_ENV")
            && !env_name.trim().is_empty()
        {
            return env_name.trim().to_string();
        }

        if let Ok(env_name) = std::env::var("APP_ENV")
            && !env_name.trim().is_empty()
        {
            return env_name.trim().to_string();
        }

        "development".to_string()
    }

    /// Returns the cascading list of `.env` file paths in ascending priority order (later files override earlier).
    pub fn get_cascaded_paths(&self) -> Vec<PathBuf> {
        if let Some(file_path) = &self.explicit_file {
            let target_path = if file_path.is_absolute() {
                file_path.clone()
            } else {
                self.base_dir.join(file_path)
            };
            return vec![target_path];
        }

        let env_name = self.resolve_active_env();
        let is_test = env_name == "test" || env_name == "testing";

        // Ascending priority: .env -> .env.{env} -> .env.local -> .env.{env}.local
        let mut paths = Vec::new();
        paths.push(self.base_dir.join(".env"));

        if !env_name.is_empty() {
            paths.push(self.base_dir.join(format!(".env.{env_name}")));
        }

        if !is_test {
            paths.push(self.base_dir.join(".env.local"));
        }

        if !env_name.is_empty() {
            paths.push(self.base_dir.join(format!(".env.{env_name}.local")));
        }

        paths
    }

    /// Discovers, reads, merges, and interpolates environment variables.
    pub fn load(&self) -> Result<HashMap<String, String>, EnvError> {
        let mut file_map = HashMap::new();

        if let Some(explicit_file) = &self.explicit_file {
            let target_path = if explicit_file.is_absolute() {
                explicit_file.clone()
            } else {
                self.base_dir.join(explicit_file)
            };

            let contents = fs::read_to_string(&target_path).map_err(|err| {
                EnvError::Io(std::io::Error::new(
                    err.kind(),
                    format!("failed to read `{}`: {err}", target_path.display()),
                ))
            })?;

            let entries = EnvParser::parse_to_map(&contents)?;
            file_map.extend(entries);
        } else {
            let paths = self.get_cascaded_paths();
            for path in paths {
                if path.is_file() {
                    let contents = match fs::read_to_string(&path) {
                        Ok(text) => text,
                        Err(err) => {
                            return Err(EnvError::Io(std::io::Error::new(
                                err.kind(),
                                format!("failed to read `{}`: {err}", path.display()),
                            )));
                        }
                    };
                    let entries = EnvParser::parse_to_map(&contents)?;
                    // Later files overwrite earlier files in ascending priority
                    file_map.extend(entries);
                }
            }
        }

        let mut merged = HashMap::new();
        let system_vars: HashMap<String, String> = std::env::vars().collect();

        if self.override_system {
            // System env provides base, .env files override
            merged.extend(system_vars);
            merged.extend(file_map);
        } else {
            // .env files provide base, system env overrides
            merged.extend(file_map);
            merged.extend(system_vars);
        }

        let interpolator = EnvInterpolator::new(&self.base_dir);
        interpolator.interpolate(&merged)
    }

    /// Loads environment variables directly from a raw `.env` string without reading the filesystem.
    pub fn load_from_str(&self, contents: &str) -> Result<HashMap<String, String>, EnvError> {
        let file_map = EnvParser::parse_to_map(contents)?;
        let mut merged = HashMap::new();
        let system_vars: HashMap<String, String> = std::env::vars().collect();

        if self.override_system {
            merged.extend(system_vars);
            merged.extend(file_map);
        } else {
            merged.extend(file_map);
            merged.extend(system_vars);
        }

        let interpolator = EnvInterpolator::new(&self.base_dir);
        interpolator.interpolate(&merged)
    }
}
