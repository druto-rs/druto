use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::error::EnvError;

/// An individual line in a `.env` file preserving structure and formatting.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditorLine {
    /// An empty line.
    Empty,
    /// A comment line starting with `#`.
    Comment(String),
    /// A key-value pair definition.
    Entry {
        /// Whether the entry is prefixed with `export `.
        has_export: bool,
        /// The variable name.
        key: String,
        /// The variable value.
        value: String,
        /// Quoting character used (`'` or `"`), if any.
        quote: Option<char>,
        /// Inline comment appended after the value, if any.
        inline_comment: Option<String>,
    },
    /// A raw line that could not be parsed into a standard entry.
    Raw(String),
}

impl EditorLine {
    /// Renders the editor line back to formatted text.
    pub fn render(&self) -> String {
        match self {
            Self::Empty => String::new(),
            Self::Comment(text) => text.clone(),
            Self::Entry {
                has_export,
                key,
                value,
                quote,
                inline_comment,
            } => {
                let mut rendered = String::new();
                if *has_export {
                    rendered.push_str("export ");
                }
                rendered.push_str(key);
                rendered.push('=');

                match quote {
                    Some(quote_char) => {
                        rendered.push(*quote_char);
                        rendered.push_str(value);
                        rendered.push(*quote_char);
                    }
                    None => {
                        rendered.push_str(value);
                    }
                }

                if let Some(comment) = inline_comment {
                    rendered.push(' ');
                    rendered.push_str(comment);
                }

                rendered
            }
            Self::Raw(text) => text.clone(),
        }
    }
}

/// Allows programmatic reading, editing, and saving of `.env` files while preserving comments,
/// empty lines, and original formatting.
#[derive(Debug, Clone)]
pub struct EnvEditor {
    file_path: Option<PathBuf>,
    lines: Vec<EditorLine>,
}

impl EnvEditor {
    /// Loads a `.env` file from the specified path for editing.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, EnvError> {
        let path_buf = path.as_ref().to_path_buf();
        let contents = fs::read_to_string(&path_buf)?;
        let mut editor = Self::parse(&contents);
        editor.file_path = Some(path_buf);
        Ok(editor)
    }

    /// Parses `.env` content from a string for editing.
    pub fn parse(contents: &str) -> Self {
        let lines_vec: Vec<&str> = contents.lines().collect();
        let mut lines = Vec::new();
        let mut line_index = 0;

        while line_index < lines_vec.len() {
            let raw_line = lines_vec[line_index];
            let trimmed = raw_line.trim_start();
            if trimmed.is_empty() {
                lines.push(EditorLine::Empty);
                line_index += 1;
                continue;
            }

            if trimmed.starts_with('#') {
                lines.push(EditorLine::Comment(raw_line.to_string()));
                line_index += 1;
                continue;
            }

            let (has_export, without_export) =
                if let Some(stripped) = trimmed.strip_prefix("export") {
                    if stripped.starts_with(char::is_whitespace) {
                        (true, stripped.trim_start())
                    } else {
                        (false, trimmed)
                    }
                } else {
                    (false, trimmed)
                };

            if let Some(equals_pos) = without_export.find('=') {
                let key = without_export[..equals_pos].trim().to_string();
                let rest = without_export[equals_pos + 1..].trim_start();

                let (value, quote, inline_comment, next_line_index) =
                    Self::parse_editor_value(rest, &lines_vec, line_index);

                lines.push(EditorLine::Entry {
                    has_export,
                    key,
                    value,
                    quote,
                    inline_comment,
                });
                line_index = next_line_index + 1;
            } else {
                lines.push(EditorLine::Raw(raw_line.to_string()));
                line_index += 1;
            }
        }

        Self {
            file_path: None,
            lines,
        }
    }
}

impl std::str::FromStr for EnvEditor {
    type Err = std::convert::Infallible;

    fn from_str(contents: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(contents))
    }
}

impl EnvEditor {
    fn parse_editor_value(
        rest: &str,
        lines: &[&str],
        current_line_index: usize,
    ) -> (String, Option<char>, Option<String>, usize) {
        if rest.is_empty() {
            return (String::new(), None, None, current_line_index);
        }

        let first = rest.chars().next().unwrap();
        if first == '\'' || first == '"' {
            let quote_char = first;
            if let Some(close_pos) = rest[1..].find(quote_char) {
                let value = rest[1..1 + close_pos].to_string();
                let remaining = rest[1 + close_pos + 1..].trim_start();
                let inline_comment = if remaining.starts_with('#') {
                    Some(remaining.to_string())
                } else {
                    None
                };
                return (value, Some(quote_char), inline_comment, current_line_index);
            }

            let mut accumulated = rest[1..].to_string();
            let mut scan_index = current_line_index + 1;
            let mut found_close = false;
            let mut inline_comment = None;

            while scan_index < lines.len() {
                let line_str = lines[scan_index];
                if let Some(close_pos) = line_str.find(quote_char) {
                    accumulated.push('\n');
                    accumulated.push_str(&line_str[..close_pos]);
                    let remaining = line_str[close_pos + 1..].trim_start();
                    if remaining.starts_with('#') {
                        inline_comment = Some(remaining.to_string());
                    }
                    found_close = true;
                    break;
                }
                accumulated.push('\n');
                accumulated.push_str(line_str);
                scan_index += 1;
            }

            if found_close {
                return (accumulated, Some(quote_char), inline_comment, scan_index);
            }
        }

        let mut value = String::new();
        let mut inline_comment = None;
        let chars: Vec<char> = rest.chars().collect();
        let mut index = 0;

        while index < chars.len() {
            if chars[index] == '#' && (index == 0 || chars[index - 1].is_whitespace()) {
                let comment_slice: String = chars[index..].iter().collect();
                inline_comment = Some(comment_slice);
                break;
            }
            value.push(chars[index]);
            index += 1;
        }

        (
            value.trim().to_string(),
            None,
            inline_comment,
            current_line_index,
        )
    }

    /// Retrieves the current value of a variable if present.
    pub fn get(&self, key: &str) -> Option<&str> {
        for line in &self.lines {
            if let EditorLine::Entry {
                key: entry_key,
                value,
                ..
            } = line
                && entry_key == key
            {
                return Some(value.as_str());
            }
        }
        None
    }

    /// Sets or updates a variable. Preserves line position if already present.
    pub fn set(&mut self, key: impl Into<String>, value: impl Into<String>) -> &mut Self {
        let key_str = key.into();
        let val_str = value.into();
        let mut found = false;

        let needs_quotes = val_str.contains(' ') || val_str.contains('#') || val_str.is_empty();

        for line in &mut self.lines {
            if let EditorLine::Entry {
                key: entry_key,
                value: entry_value,
                quote,
                ..
            } = line
                && entry_key == &key_str
            {
                *entry_value = val_str.clone();
                if quote.is_none() && needs_quotes {
                    *quote = Some('"');
                }
                found = true;
                break;
            }
        }

        if !found {
            let quote = if needs_quotes { Some('"') } else { None };
            self.lines.push(EditorLine::Entry {
                has_export: false,
                key: key_str,
                value: val_str,
                quote,
                inline_comment: None,
            });
        }

        self
    }

    /// Removes a variable by key.
    pub fn unset(&mut self, key: &str) -> &mut Self {
        self.lines.retain(|line| match line {
            EditorLine::Entry { key: entry_key, .. } => entry_key != key,
            _ => true,
        });
        self
    }

    /// Ensures that a variable exists in a companion `.env.example` file (with an empty placeholder value).
    pub fn sync_example(&self, example_path: impl AsRef<Path>, key: &str) -> Result<(), EnvError> {
        let path = example_path.as_ref();
        let mut example_editor = if path.exists() {
            Self::load(path)?
        } else {
            Self {
                file_path: Some(path.to_path_buf()),
                lines: Vec::new(),
            }
        };

        if example_editor.get(key).is_none() {
            example_editor.set(key, "");
            example_editor.save_to(path)?;
        }

        Ok(())
    }

    /// Renders the entire `.env` file back to a string.
    pub fn render(&self) -> String {
        let mut output = String::new();
        for (idx, line) in self.lines.iter().enumerate() {
            output.push_str(&line.render());
            if idx + 1 < self.lines.len() {
                output.push('\n');
            }
        }
        output
    }

    /// Saves the edited contents back to the file it was loaded from.
    pub fn save(&self) -> Result<(), EnvError> {
        match &self.file_path {
            Some(path) => self.save_to(path),
            None => Err(EnvError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "no target file path specified for saving",
            ))),
        }
    }

    /// Saves the edited contents to the specified path.
    pub fn save_to(&self, path: impl AsRef<Path>) -> Result<(), EnvError> {
        let content = self.render();
        fs::write(path, content)?;
        Ok(())
    }
}
