//! Kebab case (`kebab-case`) conversions.

use std::borrow::Cow;

use super::{is_delimited_ascii, join_words_cased, split_words};

/// Checks if the input slice is already in valid `kebab-case` without allocating.
///
/// A valid `kebab-case` string is non-empty, contains only lowercase ASCII letters,
/// digits, and single hyphen separators, with no leading or trailing hyphens.
#[inline]
pub fn is_kebab_case(input: &str) -> bool {
    is_delimited_ascii(input, b'-', false)
}

/// Converts a string slice to `kebab-case`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `kebab-case`.
pub fn to_kebab_case(input: &str) -> Cow<'_, str> {
    if is_kebab_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(join_words_cased(&words, '-', false, input.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_kebab_case() {
        assert!(is_kebab_case("user-id"));
        assert!(is_kebab_case("foo-bar-baz"));
        assert!(is_kebab_case("v1-api-endpoint"));

        assert!(!is_kebab_case(""));
        assert!(!is_kebab_case("-user-id"));
        assert!(!is_kebab_case("user-id-"));
        assert!(!is_kebab_case("user--id"));
        assert!(!is_kebab_case("userId"));
        assert!(!is_kebab_case("user_id"));
    }

    #[test]
    fn test_to_kebab_case_borrowed() {
        let input = "user-profile-id";
        let result = to_kebab_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "user-profile-id");
    }

    #[test]
    fn test_to_kebab_case_conversions() {
        assert_eq!(to_kebab_case("userId"), "user-id");
        assert_eq!(to_kebab_case("UserId"), "user-id");
        assert_eq!(to_kebab_case("user_id"), "user-id");
        assert_eq!(to_kebab_case("HTTPResponse"), "http-response");
        assert_eq!(to_kebab_case("--hello--world--"), "hello-world");
        assert_eq!(to_kebab_case(""), "");
    }
}
