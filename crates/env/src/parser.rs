use std::collections::HashMap;

use crate::error::EnvError;

/// A parsed key-value entry from a `.env` source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedEntry {
    /// The variable name.
    pub key: String,
    /// The parsed raw value.
    pub value: String,
    /// The line number where the entry began.
    pub line: usize,
}

/// Parses `.env` formatted content into key-value entries.
pub struct EnvParser;

impl EnvParser {
    /// Parses raw `.env` string contents into a list of parsed entries.
    pub fn parse(input: &str) -> Result<Vec<ParsedEntry>, EnvError> {
        let lines: Vec<&str> = input.lines().collect();
        let mut entries = Vec::new();
        let mut line_index = 0;

        while line_index < lines.len() {
            let current_line = lines[line_index];
            let trimmed = current_line.trim_start();
            let entry_line_number = line_index + 1;

            if trimmed.is_empty() || trimmed.starts_with('#') {
                line_index += 1;
                continue;
            }

            let without_export = if let Some(stripped) = trimmed.strip_prefix("export") {
                if stripped.starts_with(char::is_whitespace) {
                    stripped.trim_start()
                } else {
                    trimmed
                }
            } else {
                trimmed
            };

            let equals_pos = match without_export.find('=') {
                Some(pos) => pos,
                None => {
                    return Err(EnvError::Parse {
                        line: entry_line_number,
                        message: format!(
                            "missing '=' assignment separator in line: `{current_line}`"
                        ),
                    });
                }
            };

            let raw_key = without_export[..equals_pos].trim();
            if raw_key.is_empty() {
                return Err(EnvError::Parse {
                    line: entry_line_number,
                    message: "variable name cannot be empty".to_string(),
                });
            }

            let rest_after_equals = without_export[equals_pos + 1..].trim_start();

            let (value, next_line_index) =
                Self::parse_value(rest_after_equals, &lines, line_index, entry_line_number)?;

            entries.push(ParsedEntry {
                key: raw_key.to_string(),
                value,
                line: entry_line_number,
            });

            line_index = next_line_index + 1;
        }

        Ok(entries)
    }

    /// Parses entries directly into a HashMap.
    pub fn parse_to_map(input: &str) -> Result<HashMap<String, String>, EnvError> {
        let entries = Self::parse(input)?;
        let mut map = HashMap::with_capacity(entries.len());
        for entry in entries {
            map.insert(entry.key, entry.value);
        }
        Ok(map)
    }

    fn parse_value(
        initial_value_slice: &str,
        lines: &[&str],
        current_line_index: usize,
        entry_line_number: usize,
    ) -> Result<(String, usize), EnvError> {
        if initial_value_slice.is_empty() {
            return Ok((String::new(), current_line_index));
        }

        let first_char = initial_value_slice.chars().next().unwrap();

        if first_char == '\'' {
            Self::parse_single_quoted(
                &initial_value_slice[1..],
                lines,
                current_line_index,
                entry_line_number,
            )
        } else if first_char == '"' {
            Self::parse_double_quoted(
                &initial_value_slice[1..],
                lines,
                current_line_index,
                entry_line_number,
            )
        } else {
            let unquoted = Self::parse_unquoted(initial_value_slice);
            Ok((unquoted, current_line_index))
        }
    }

    fn parse_single_quoted<'a>(
        mut slice: &'a str,
        lines: &[&'a str],
        mut line_index: usize,
        entry_line_number: usize,
    ) -> Result<(String, usize), EnvError> {
        let mut accumulated = String::new();

        loop {
            if let Some(closing_pos) = slice.find('\'') {
                accumulated.push_str(&slice[..closing_pos]);
                return Ok((accumulated, line_index));
            }

            accumulated.push_str(slice);
            accumulated.push('\n');

            line_index += 1;
            if line_index >= lines.len() {
                return Err(EnvError::Parse {
                    line: entry_line_number,
                    message: "unterminated single-quoted string".to_string(),
                });
            }

            slice = lines[line_index];
        }
    }

    fn parse_double_quoted<'a>(
        mut slice: &'a str,
        lines: &[&'a str],
        mut line_index: usize,
        entry_line_number: usize,
    ) -> Result<(String, usize), EnvError> {
        let mut accumulated = String::new();
        let mut in_escape = false;

        loop {
            let chars = slice.chars();
            for ch in chars {
                if in_escape {
                    match ch {
                        'n' => accumulated.push('\n'),
                        'r' => accumulated.push('\r'),
                        't' => accumulated.push('\t'),
                        '\\' => accumulated.push('\\'),
                        '"' => accumulated.push('"'),
                        '$' => accumulated.push_str("\\$"),
                        other => {
                            accumulated.push('\\');
                            accumulated.push(other);
                        }
                    }
                    in_escape = false;
                } else if ch == '\\' {
                    in_escape = true;
                } else if ch == '"' {
                    return Ok((accumulated, line_index));
                } else {
                    accumulated.push(ch);
                }
            }

            accumulated.push('\n');
            line_index += 1;
            if line_index >= lines.len() {
                return Err(EnvError::Parse {
                    line: entry_line_number,
                    message: "unterminated double-quoted string".to_string(),
                });
            }

            slice = lines[line_index];
        }
    }

    fn parse_unquoted(slice: &str) -> String {
        let mut result = String::new();
        let chars: Vec<char> = slice.chars().collect();
        let mut index = 0;

        while index < chars.len() {
            let ch = chars[index];
            if ch == '#' && (index == 0 || chars[index - 1].is_whitespace()) {
                break;
            }
            result.push(ch);
            index += 1;
        }

        result.trim().to_string()
    }
}
