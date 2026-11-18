//! Pascal case (`PascalCase`) conversions.

use std::borrow::Cow;

use super::{is_camel_or_pascal, join_words_camel_or_pascal, split_words};

/// Checks if the input slice is already in valid `PascalCase` without allocating.
///
/// A valid `PascalCase` string is non-empty, begins with an uppercase letter,
/// contains only alphanumeric characters, and avoids consecutive uppercase characters.
#[inline]
pub fn is_pascal_case(input: &str) -> bool {
    is_camel_or_pascal(input, true)
}

/// Converts a string slice to `PascalCase`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `PascalCase`.
pub fn to_pascal_case(input: &str) -> Cow<'_, str> {
    if is_pascal_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(join_words_camel_or_pascal(&words, true, input.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_pascal_case() {
        assert!(is_pascal_case("UserId"));
        assert!(is_pascal_case("FooBarBaz"));
        assert!(is_pascal_case("User123Profile"));
        assert!(is_pascal_case("HttpResponse"));

        assert!(!is_pascal_case(""));
        assert!(!is_pascal_case("userId"));
        assert!(!is_pascal_case("user_id"));
        assert!(!is_pascal_case("HTTPResponse"));
        assert!(!is_pascal_case("Foo_Bar"));
    }

    #[test]
    fn test_to_pascal_case_borrowed() {
        let input = "UserProfile";
        let result = to_pascal_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "UserProfile");
    }

    #[test]
    fn test_to_pascal_case_conversions() {
        assert_eq!(to_pascal_case("user_id"), "UserId");
        assert_eq!(to_pascal_case("user-id"), "UserId");
        assert_eq!(to_pascal_case("userId"), "UserId");
        assert_eq!(to_pascal_case("HTTPResponse"), "HttpResponse");
        assert_eq!(to_pascal_case("foo bar baz"), "FooBarBaz");
        assert_eq!(to_pascal_case(""), "");
    }
}
