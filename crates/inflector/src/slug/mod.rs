//! URL slug generation with Unicode transliteration and configurable delimiter options.

use std::collections::HashMap;

mod transliterate;

use transliterate::transliterate_char;

/// Configuration options for URL slug generation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct SlugOptions {
    /// Delimiter separating words (default: `'-'`).
    pub delimiter: char,
    /// Whether to convert output to lowercase (default: `true`).
    pub lowercase: bool,
    /// Whether to preserve non-ASCII alphanumeric characters (default: `false`).
    pub allow_unicode: bool,
    /// Whether to expand common symbols like `&` to "and" and `@` to "at" (default: `true`).
    pub symbol_expansion: bool,
    /// Custom character replacement map.
    pub custom_replacements: HashMap<char, String>,
}

impl Default for SlugOptions {
    fn default() -> Self {
        Self {
            delimiter: '-',
            lowercase: true,
            allow_unicode: false,
            symbol_expansion: true,
            custom_replacements: HashMap::new(),
        }
    }
}

impl SlugOptions {
    /// Creates a default `SlugOptions` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the word separator delimiter.
    pub fn with_delimiter(mut self, delimiter: char) -> Self {
        self.delimiter = delimiter;
        self
    }

    /// Configures lowercase conversion.
    pub fn with_lowercase(mut self, lowercase: bool) -> Self {
        self.lowercase = lowercase;
        self
    }

    /// Configures whether non-ASCII alphanumeric characters are preserved.
    pub fn with_allow_unicode(mut self, allow: bool) -> Self {
        self.allow_unicode = allow;
        self
    }

    /// Configures whether symbols like `&` and `@` are expanded into words.
    pub fn with_symbol_expansion(mut self, expand: bool) -> Self {
        self.symbol_expansion = expand;
        self
    }

    /// Adds a custom character replacement rule.
    pub fn with_replacement(mut self, from: char, to: impl Into<String>) -> Self {
        self.custom_replacements.insert(from, to.into());
        self
    }
}

/// Generates a URL-safe slug from a string slice with default options.
///
/// # Examples
/// ```
/// use inflector::slug::slugify;
///
/// assert_eq!(slugify("Hello World & Deep Dive!"), "hello-world-and-deep-dive");
/// assert_eq!(slugify("Café Déjà Vu"), "cafe-deja-vu");
/// ```
pub fn slugify(input: &str) -> String {
    slugify_with_options(input, &SlugOptions::default())
}

/// Generates a URL-safe slug with customized options.
pub fn slugify_with_options(input: &str, options: &SlugOptions) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut intermediate = String::with_capacity(input.len() + 8);

    for current_char in input.chars() {
        if let Some(replacement) = options.custom_replacements.get(&current_char) {
            intermediate.push(options.delimiter);
            intermediate.push_str(replacement);
            intermediate.push(options.delimiter);
        } else if options.symbol_expansion && current_char == '&' {
            intermediate.push(options.delimiter);
            intermediate.push_str("and");
            intermediate.push(options.delimiter);
        } else if options.symbol_expansion && current_char == '@' {
            intermediate.push(options.delimiter);
            intermediate.push_str("at");
            intermediate.push(options.delimiter);
        } else if let Some(transliterated) = transliterate_char(current_char) {
            intermediate.push_str(transliterated);
        } else if current_char.is_ascii_alphanumeric()
            || (options.allow_unicode && current_char.is_alphanumeric())
        {
            intermediate.push(current_char);
        } else {
            intermediate.push(options.delimiter);
        }
    }

    // Collapse consecutive delimiters and trim leading/trailing delimiters
    let mut result = String::with_capacity(intermediate.len());
    let mut previous_was_delimiter = true;

    for current_char in intermediate.chars() {
        if current_char == options.delimiter {
            if !previous_was_delimiter {
                result.push(options.delimiter);
                previous_was_delimiter = true;
            }
        } else {
            previous_was_delimiter = false;
            if options.lowercase {
                for lower_char in current_char.to_lowercase() {
                    result.push(lower_char);
                }
            } else {
                result.push(current_char);
            }
        }
    }

    if result.ends_with(options.delimiter) {
        result.pop();
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slugify_defaults() {
        assert_eq!(
            slugify("Hello World & Deep Dive!"),
            "hello-world-and-deep-dive"
        );
        assert_eq!(slugify("Café Déjà Vu"), "cafe-deja-vu");
        assert_eq!(slugify("user@example.com"), "user-at-example-com");
        assert_eq!(slugify("___hello___world___"), "hello-world");
        assert_eq!(slugify(""), "");
    }

    #[test]
    fn test_slugify_custom_options() {
        let options = SlugOptions::new()
            .with_delimiter('_')
            .with_lowercase(false)
            .with_replacement('%', "percent");

        assert_eq!(slugify_with_options("50% Off", &options), "50_percent_Off");
    }

    #[test]
    fn test_slugify_unicode_retention() {
        let options = SlugOptions::new().with_allow_unicode(true);
        assert_eq!(
            slugify_with_options("Hello 東京 World", &options),
            "hello-東京-world"
        );
    }
}
