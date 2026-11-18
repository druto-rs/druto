use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::error::EnvError;

/// Resolves variable references (`${VAR}`, `$VAR`, `${VAR:-default}`) and custom resolvers.
pub struct EnvInterpolator {
    base_path: PathBuf,
}

impl EnvInterpolator {
    /// Creates a new interpolator with the given base directory for relative path resolution.
    pub fn new(base_path: impl Into<PathBuf>) -> Self {
        Self {
            base_path: base_path.into(),
        }
    }

    /// Expands all variables in the provided key-value map.
    pub fn interpolate(
        &self,
        map: &HashMap<String, String>,
    ) -> Result<HashMap<String, String>, EnvError> {
        let mut resolved = HashMap::with_capacity(map.len());
        let mut cache = HashMap::new();

        for key in map.keys() {
            let mut stack = Vec::new();
            let value = self.resolve_variable(key, map, &mut stack, &mut cache)?;
            resolved.insert(key.clone(), value);
        }

        Ok(resolved)
    }

    fn resolve_variable(
        &self,
        key: &str,
        map: &HashMap<String, String>,
        stack: &mut Vec<String>,
        cache: &mut HashMap<String, String>,
    ) -> Result<String, EnvError> {
        if let Some(cached_value) = cache.get(key) {
            return Ok(cached_value.clone());
        }

        if stack.contains(&key.to_string()) {
            let mut cycle = stack.clone();
            cycle.push(key.to_string());
            return Err(EnvError::CyclicDependency { cycle });
        }

        stack.push(key.to_string());

        let raw_value = match map.get(key) {
            Some(value) => value.clone(),
            None => std::env::var(key).unwrap_or_default(),
        };

        let expanded = self.expand_string(&raw_value, map, stack, cache)?;
        let processed = self.process_resolvers(key, expanded, &self.base_path)?;

        stack.pop();
        cache.insert(key.to_string(), processed.clone());
        Ok(processed)
    }

    fn expand_string(
        &self,
        input: &str,
        map: &HashMap<String, String>,
        stack: &mut Vec<String>,
        cache: &mut HashMap<String, String>,
    ) -> Result<String, EnvError> {
        let mut result = String::new();
        let chars: Vec<char> = input.chars().collect();
        let mut index = 0;

        while index < chars.len() {
            if chars[index] == '\\' && index + 1 < chars.len() && chars[index + 1] == '$' {
                result.push('$');
                index += 2;
                continue;
            }

            if chars[index] == '$' {
                if index + 1 < chars.len() && chars[index + 1] == '{' {
                    let start_brace = index + 2;
                    let mut end_brace = None;
                    let mut depth = 1;
                    let mut scan_index = start_brace;

                    while scan_index < chars.len() {
                        if chars[scan_index] == '{' {
                            depth += 1;
                        } else if chars[scan_index] == '}' {
                            depth -= 1;
                            if depth == 0 {
                                end_brace = Some(scan_index);
                                break;
                            }
                        }
                        scan_index += 1;
                    }

                    if let Some(close_pos) = end_brace {
                        let inner: String = chars[start_brace..close_pos].iter().collect();
                        index = close_pos + 1;

                        if let Some(separator_pos) = inner.find(":-") {
                            let var_name = inner[..separator_pos].trim();
                            let fallback_raw = &inner[separator_pos + 2..];

                            let value = self.resolve_variable(var_name, map, stack, cache)?;
                            if !value.is_empty() {
                                result.push_str(&value);
                            } else {
                                let fallback_expanded =
                                    self.expand_string(fallback_raw, map, stack, cache)?;
                                result.push_str(&fallback_expanded);
                            }
                        } else {
                            let var_name = inner.trim();
                            let value = self.resolve_variable(var_name, map, stack, cache)?;
                            result.push_str(&value);
                        }
                        continue;
                    }
                } else if index + 1 < chars.len()
                    && (chars[index + 1].is_ascii_alphabetic() || chars[index + 1] == '_')
                {
                    let start_var = index + 1;
                    let mut end_var = start_var;
                    while end_var < chars.len()
                        && (chars[end_var].is_ascii_alphanumeric() || chars[end_var] == '_')
                    {
                        end_var += 1;
                    }

                    let var_name: String = chars[start_var..end_var].iter().collect();
                    index = end_var;

                    let value = self.resolve_variable(&var_name, map, stack, cache)?;
                    result.push_str(&value);
                    continue;
                }
            }

            result.push(chars[index]);
            index += 1;
        }

        Ok(result)
    }

    fn process_resolvers(
        &self,
        key: &str,
        value: String,
        base_path: &Path,
    ) -> Result<String, EnvError> {
        if let Some(file_path_str) = value.strip_prefix("file:") {
            let relative_or_absolute = Path::new(file_path_str.trim());
            let target_path = if relative_or_absolute.is_absolute() {
                relative_or_absolute.to_path_buf()
            } else {
                base_path.join(relative_or_absolute)
            };

            match fs::read_to_string(&target_path) {
                Ok(contents) => {
                    let trimmed = contents
                        .strip_suffix("\r\n")
                        .or_else(|| contents.strip_suffix('\n'))
                        .unwrap_or(&contents);
                    Ok(trimmed.to_string())
                }
                Err(err) => Err(EnvError::Resolver {
                    key: key.to_string(),
                    message: format!(
                        "unable to read secret file `{}`: {err}",
                        target_path.display()
                    ),
                }),
            }
        } else {
            Ok(value)
        }
    }
}
