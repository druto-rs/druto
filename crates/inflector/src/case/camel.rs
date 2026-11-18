//! Camel case (`camelCase`) conversions.

use std::borrow::Cow;

use super::{is_camel_or_pascal, join_words_camel_or_pascal, split_words};

/// Checks if the input slice is already in valid `camelCase` without allocating.
///
/// A valid `camelCase` string is non-empty, begins with a lowercase letter,
/// contains only alphanumeric characters, and avoids consecutive uppercase characters.
#[inline]
pub fn is_camel_case(input: &str) -> bool {
    is_camel_or_pascal(input, false)
}

/// Converts a string slice to `camelCase`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `camelCase`.
pub fn to_camel_case(input: &str) -> Cow<'_, str> {
    if is_camel_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(join_words_camel_or_pascal(&words, false, input.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_camel_case() {
        assert!(is_camel_case("userId"));
        assert!(is_camel_case("fooBarBaz"));
        assert!(is_camel_case("user123Profile"));
        assert!(is_camel_case("httpResponse"));

        assert!(!is_camel_case(""));
        assert!(!is_camel_case("UserId"));
        assert!(!is_camel_case("user_id"));
        assert!(!is_camel_case("HTTPResponse"));
        assert!(!is_camel_case("foo_bar"));
    }

    #[test]
    fn test_to_camel_case_borrowed() {
        let input = "userProfile";
        let result = to_camel_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "userProfile");
    }

    #[test]
    fn test_to_camel_case_conversions() {
        assert_eq!(to_camel_case("user_id"), "userId");
        assert_eq!(to_camel_case("user-id"), "userId");
        assert_eq!(to_camel_case("UserId"), "userId");
        assert_eq!(to_camel_case("HTTPResponse"), "httpResponse");
        assert_eq!(to_camel_case("foo bar baz"), "fooBarBaz");
        assert_eq!(to_camel_case(""), "");
    }
}
