//! Extension trait providing fluent methods on string slices.

use std::borrow::Cow;

use crate::{
    case::{
        classify, foreign_key, is_camel_case, is_kebab_case, is_pascal_case,
        is_screaming_snake_case, is_sentence_case, is_snake_case, is_title_case, is_train_case,
        tableize, to_camel_case, to_kebab_case, to_pascal_case, to_screaming_snake_case,
        to_sentence_case, to_snake_case, to_title_case, to_train_case,
    },
    grammar::{
        is_plural, is_singular, pluralize, pluralize_with_count, pluralize_with_count_inclusive,
        singularize,
    },
    slug::{SlugOptions, slugify, slugify_with_options},
    utils::{
        ExcerptOptions, TruncateOptions, excerpt, finish, mask, start, truncate, unwrap, words,
        wrap,
    },
};

/// Extension trait enabling fluent inflections, casing transformations, and utilities on `&str`.
pub trait StrExt {
    /// Converts to `snake_case`.
    fn to_snake_case(&self) -> Cow<'_, str>;

    /// Converts to `SCREAMING_SNAKE_CASE`.
    fn to_screaming_snake_case(&self) -> Cow<'_, str>;

    /// Converts to `camelCase`.
    fn to_camel_case(&self) -> Cow<'_, str>;

    /// Converts to `PascalCase`.
    fn to_pascal_case(&self) -> Cow<'_, str>;

    /// Converts to `kebab-case`.
    fn to_kebab_case(&self) -> Cow<'_, str>;

    /// Converts to `Train-Case`.
    fn to_train_case(&self) -> Cow<'_, str>;

    /// Converts to `Title Case`.
    fn to_title_case(&self) -> Cow<'_, str>;

    /// Converts to `Sentence case`.
    fn to_sentence_case(&self) -> Cow<'_, str>;

    /// Converts model name to database table name.
    fn tableize(&self) -> String;

    /// Converts table name to model class name.
    fn classify(&self) -> String;

    /// Generates foreign key column name.
    fn foreign_key(&self) -> String;

    /// Checks if slice is valid `snake_case`.
    fn is_snake_case(&self) -> bool;

    /// Checks if slice is valid `SCREAMING_SNAKE_CASE`.
    fn is_screaming_snake_case(&self) -> bool;

    /// Checks if slice is valid `camelCase`.
    fn is_camel_case(&self) -> bool;

    /// Checks if slice is valid `PascalCase`.
    fn is_pascal_case(&self) -> bool;

    /// Checks if slice is valid `kebab-case`.
    fn is_kebab_case(&self) -> bool;

    /// Checks if slice is valid `Train-Case`.
    fn is_train_case(&self) -> bool;

    /// Checks if slice is valid `Title Case`.
    fn is_title_case(&self) -> bool;

    /// Checks if slice is valid `Sentence case`.
    fn is_sentence_case(&self) -> bool;

    /// Pluralizes word.
    fn to_plural(&self) -> String;

    /// Singularizes word.
    fn to_singular(&self) -> String;

    /// Inflects based on count.
    fn pluralize_with_count(&self, count: usize) -> String;

    /// Inflects with count prefix.
    fn pluralize_with_count_inclusive(&self, count: usize) -> String;

    /// Checks if word is plural.
    fn is_plural(&self) -> bool;

    /// Checks if word is singular.
    fn is_singular(&self) -> bool;

    /// Converts to URL-safe slug.
    fn to_slug(&self) -> String;

    /// Converts to URL-safe slug with options.
    fn to_slug_with_options(&self, options: &SlugOptions) -> String;

    /// Truncates to maximum characters with default ellipsis.
    fn truncate(&self, max_characters: usize) -> String;

    /// Truncates to maximum characters with custom options.
    fn truncate_with_options(&self, max_characters: usize, options: &TruncateOptions) -> String;

    /// Limits word count.
    fn words(&self, max_words: usize, suffix: &str) -> String;

    /// Extracts surrounding context excerpt.
    fn excerpt(&self, term: &str, options: &ExcerptOptions) -> String;

    /// Masks substring with character.
    fn mask(&self, mask_char: char, start: usize, length: Option<usize>) -> String;

    /// Ensures prefix.
    fn start(&self, prefix: &str) -> String;

    /// Ensures suffix.
    fn finish(&self, suffix: &str) -> String;

    /// Wraps with prefix and suffix.
    fn wrap(&self, before: &str, after: &str) -> String;

    /// Removes enclosing prefix and suffix if present.
    fn unwrap<'a>(&'a self, before: &str, after: &str) -> &'a str;
}

impl StrExt for str {
    fn to_snake_case(&self) -> Cow<'_, str> {
        to_snake_case(self)
    }

    fn to_screaming_snake_case(&self) -> Cow<'_, str> {
        to_screaming_snake_case(self)
    }

    fn to_camel_case(&self) -> Cow<'_, str> {
        to_camel_case(self)
    }

    fn to_pascal_case(&self) -> Cow<'_, str> {
        to_pascal_case(self)
    }

    fn to_kebab_case(&self) -> Cow<'_, str> {
        to_kebab_case(self)
    }

    fn to_train_case(&self) -> Cow<'_, str> {
        to_train_case(self)
    }

    fn to_title_case(&self) -> Cow<'_, str> {
        to_title_case(self)
    }

    fn to_sentence_case(&self) -> Cow<'_, str> {
        to_sentence_case(self)
    }

    fn tableize(&self) -> String {
        tableize(self)
    }

    fn classify(&self) -> String {
        classify(self)
    }

    fn foreign_key(&self) -> String {
        foreign_key(self)
    }

    fn is_snake_case(&self) -> bool {
        is_snake_case(self)
    }

    fn is_screaming_snake_case(&self) -> bool {
        is_screaming_snake_case(self)
    }

    fn is_camel_case(&self) -> bool {
        is_camel_case(self)
    }

    fn is_pascal_case(&self) -> bool {
        is_pascal_case(self)
    }

    fn is_kebab_case(&self) -> bool {
        is_kebab_case(self)
    }

    fn is_train_case(&self) -> bool {
        is_train_case(self)
    }

    fn is_title_case(&self) -> bool {
        is_title_case(self)
    }

    fn is_sentence_case(&self) -> bool {
        is_sentence_case(self)
    }

    fn to_plural(&self) -> String {
        pluralize(self)
    }

    fn to_singular(&self) -> String {
        singularize(self)
    }

    fn pluralize_with_count(&self, count: usize) -> String {
        pluralize_with_count(self, count)
    }

    fn pluralize_with_count_inclusive(&self, count: usize) -> String {
        pluralize_with_count_inclusive(self, count)
    }

    fn is_plural(&self) -> bool {
        is_plural(self)
    }

    fn is_singular(&self) -> bool {
        is_singular(self)
    }

    fn to_slug(&self) -> String {
        slugify(self)
    }

    fn to_slug_with_options(&self, options: &SlugOptions) -> String {
        slugify_with_options(self, options)
    }

    fn truncate(&self, max_characters: usize) -> String {
        truncate(self, max_characters, &TruncateOptions::default())
    }

    fn truncate_with_options(&self, max_characters: usize, options: &TruncateOptions) -> String {
        truncate(self, max_characters, options)
    }

    fn words(&self, max_words: usize, suffix: &str) -> String {
        words(self, max_words, suffix)
    }

    fn excerpt(&self, term: &str, options: &ExcerptOptions) -> String {
        excerpt(self, term, options)
    }

    fn mask(&self, mask_char: char, start_idx: usize, length: Option<usize>) -> String {
        mask(self, mask_char, start_idx, length)
    }

    fn start(&self, prefix: &str) -> String {
        start(self, prefix)
    }

    fn finish(&self, suffix: &str) -> String {
        finish(self, suffix)
    }

    fn wrap(&self, before: &str, after: &str) -> String {
        wrap(self, before, after)
    }

    fn unwrap<'a>(&'a self, before: &str, after: &str) -> &'a str {
        unwrap(self, before, after)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_str_ext_fluent_methods() {
        assert_eq!("user_profile".to_camel_case(), "userProfile");
        assert_eq!("user_profile".tableize(), "user_profiles");
        assert_eq!("user_profiles".classify(), "UserProfile");
        assert_eq!("user".to_plural(), "users");
        assert_eq!("users".to_singular(), "user");
        assert_eq!("Hello World".to_slug(), "hello-world");
        assert_eq!("123456".mask('*', 2, Some(2)), "12**56");
        assert_eq!("endpoint".finish("/"), "endpoint/");
        assert_eq!("/endpoint".start("/"), "/endpoint");
    }
}
