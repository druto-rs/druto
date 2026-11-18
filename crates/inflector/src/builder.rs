//! Fluent, chainable `StrBuilder` for composing sequential string transformations.

use std::{fmt, ops::Deref};

use crate::{
    case::{
        classify, foreign_key, tableize, to_camel_case, to_kebab_case, to_pascal_case,
        to_screaming_snake_case, to_sentence_case, to_snake_case, to_title_case, to_train_case,
    },
    grammar::{pluralize, pluralize_with_count, singularize},
    slug::{SlugOptions, slugify, slugify_with_options},
    utils::{TruncateOptions, finish, mask, start, truncate, words, wrap},
};

/// Fluent string transformation pipeline.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct StrBuilder {
    value: String,
}

impl StrBuilder {
    /// Creates a new `StrBuilder` from an initial string or slice.
    pub fn new(initial: impl Into<String>) -> Self {
        Self {
            value: initial.into(),
        }
    }

    /// Transforms the inner string into `snake_case`.
    pub fn snake_case(mut self) -> Self {
        self.value = to_snake_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `SCREAMING_SNAKE_CASE`.
    pub fn screaming_snake_case(mut self) -> Self {
        self.value = to_screaming_snake_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `camelCase`.
    pub fn camel_case(mut self) -> Self {
        self.value = to_camel_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `PascalCase`.
    pub fn pascal_case(mut self) -> Self {
        self.value = to_pascal_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `kebab-case`.
    pub fn kebab_case(mut self) -> Self {
        self.value = to_kebab_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `Train-Case`.
    pub fn train_case(mut self) -> Self {
        self.value = to_train_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `Title Case`.
    pub fn title_case(mut self) -> Self {
        self.value = to_title_case(&self.value).into_owned();
        self
    }

    /// Transforms the inner string into `Sentence case`.
    pub fn sentence_case(mut self) -> Self {
        self.value = to_sentence_case(&self.value).into_owned();
        self
    }

    /// Converts a model name into a conventional table name.
    pub fn tableize(mut self) -> Self {
        self.value = tableize(&self.value);
        self
    }

    /// Converts a table name into a conventional model class name.
    pub fn classify(mut self) -> Self {
        self.value = classify(&self.value);
        self
    }

    /// Generates a foreign key column name.
    pub fn foreign_key(mut self) -> Self {
        self.value = foreign_key(&self.value);
        self
    }

    /// Pluralizes the inner string.
    pub fn pluralize(mut self) -> Self {
        self.value = pluralize(&self.value);
        self
    }

    /// Singularizes the inner string.
    pub fn singularize(mut self) -> Self {
        self.value = singularize(&self.value);
        self
    }

    /// Inflects based on the given count.
    pub fn pluralize_with_count(mut self, count: usize) -> Self {
        self.value = pluralize_with_count(&self.value, count);
        self
    }

    /// Converts the inner string to a URL-safe slug.
    pub fn slugify(mut self) -> Self {
        self.value = slugify(&self.value);
        self
    }

    /// Converts the inner string to a URL-safe slug with options.
    pub fn slugify_with_options(mut self, options: &SlugOptions) -> Self {
        self.value = slugify_with_options(&self.value, options);
        self
    }

    /// Truncates the inner string to maximum characters with default ellipsis.
    pub fn truncate(mut self, max_characters: usize) -> Self {
        self.value = truncate(&self.value, max_characters, &TruncateOptions::default());
        self
    }

    /// Truncates the inner string with custom options.
    pub fn truncate_with_options(
        mut self,
        max_characters: usize,
        options: &TruncateOptions,
    ) -> Self {
        self.value = truncate(&self.value, max_characters, options);
        self
    }

    /// Limits the inner string to a maximum number of words.
    pub fn words(mut self, max_words: usize, suffix: &str) -> Self {
        self.value = words(&self.value, max_words, suffix);
        self
    }

    /// Masks a substring of the inner string with the given character.
    pub fn mask(mut self, mask_char: char, start: usize, length: Option<usize>) -> Self {
        self.value = mask(&self.value, mask_char, start, length);
        self
    }

    /// Ensures the inner string starts with the given prefix.
    pub fn start(mut self, prefix: &str) -> Self {
        self.value = start(&self.value, prefix);
        self
    }

    /// Ensures the inner string ends with the given suffix.
    pub fn finish(mut self, suffix: &str) -> Self {
        self.value = finish(&self.value, suffix);
        self
    }

    /// Wraps the inner string with prefix and suffix strings.
    pub fn wrap(mut self, before: &str, after: &str) -> Self {
        self.value = wrap(&self.value, before, after);
        self
    }

    /// Consumes the builder and returns the underlying `String`.
    pub fn into_string(self) -> String {
        self.value
    }

    /// Returns a string slice of the current builder value.
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

impl Deref for StrBuilder {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl AsRef<str> for StrBuilder {
    fn as_ref(&self) -> &str {
        &self.value
    }
}

impl fmt::Display for StrBuilder {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.value)
    }
}

impl From<String> for StrBuilder {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&str> for StrBuilder {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builder_chaining() {
        let result = StrBuilder::new("UserProfile")
            .snake_case()
            .pluralize()
            .into_string();
        assert_eq!(result, "user_profiles");

        let chained = StrBuilder::new("user_account_id")
            .camel_case()
            .finish("Updated")
            .into_string();
        assert_eq!(chained, "userAccountIdUpdated");
    }
}
