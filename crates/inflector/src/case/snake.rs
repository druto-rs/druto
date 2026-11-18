//! Snake case (`snake_case`) and screaming snake case (`SCREAMING_SNAKE_CASE`) conversions.

use std::borrow::Cow;

use super::{is_delimited_ascii, join_words_cased, split_words};

/// Checks if the input slice is already in valid `snake_case` without allocating.
///
/// A valid `snake_case` string is non-empty, contains only lowercase ASCII letters,
/// digits, and single underscore separators, with no leading or trailing underscores.
#[inline]
pub fn is_snake_case(input: &str) -> bool {
    is_delimited_ascii(input, b'_', false)
}

/// Checks if the input slice is already in valid `SCREAMING_SNAKE_CASE` without allocating.
#[inline]
pub fn is_screaming_snake_case(input: &str) -> bool {
    is_delimited_ascii(input, b'_', true)
}

/// Converts a string slice to `snake_case`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `snake_case`.
pub fn to_snake_case(input: &str) -> Cow<'_, str> {
    if is_snake_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(join_words_cased(&words, '_', false, input.len()))
}

/// Converts a string slice to `SCREAMING_SNAKE_CASE`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `SCREAMING_SNAKE_CASE`.
pub fn to_screaming_snake_case(input: &str) -> Cow<'_, str> {
    if is_screaming_snake_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(join_words_cased(&words, '_', true, input.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_snake_case() {
        assert!(is_snake_case("user_id"));
        assert!(is_snake_case("foo_bar_baz"));
        assert!(is_snake_case("user_123_profile"));
        assert!(is_snake_case("status"));

        assert!(!is_snake_case(""));
        assert!(!is_snake_case("_user_id"));
        assert!(!is_snake_case("user_id_"));
        assert!(!is_snake_case("user__id"));
        assert!(!is_snake_case("userId"));
        assert!(!is_snake_case("USER_ID"));
        assert!(!is_snake_case("user-id"));
    }

    #[test]
    fn test_is_screaming_snake_case() {
        assert!(is_screaming_snake_case("USER_ID"));
        assert!(is_screaming_snake_case("FOO_BAR_BAZ"));
        assert!(is_screaming_snake_case("HTTP_200_OK"));
        assert!(is_screaming_snake_case("STATUS"));

        assert!(!is_screaming_snake_case(""));
        assert!(!is_screaming_snake_case("_USER_ID"));
        assert!(!is_screaming_snake_case("USER_ID_"));
        assert!(!is_screaming_snake_case("USER__ID"));
        assert!(!is_screaming_snake_case("userId"));
        assert!(!is_screaming_snake_case("user_id"));
    }

    #[test]
    fn test_to_snake_case_borrowed_optimization() {
        let input = "user_account_id";
        let result = to_snake_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "user_account_id");
    }

    #[test]
    fn test_to_snake_case_conversions() {
        assert_eq!(to_snake_case("userId"), "user_id");
        assert_eq!(to_snake_case("UserId"), "user_id");
        assert_eq!(to_snake_case("user-id"), "user_id");
        assert_eq!(to_snake_case("HTTPResponse"), "http_response");
        assert_eq!(
            to_snake_case("getHTTPResponseCode"),
            "get_http_response_code"
        );
        assert_eq!(to_snake_case("__hello__world__"), "hello_world");
        assert_eq!(to_snake_case(""), "");
    }

    #[test]
    fn test_to_screaming_snake_case() {
        let input = "USER_ACCOUNT_ID";
        let result = to_screaming_snake_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));

        assert_eq!(to_screaming_snake_case("userId"), "USER_ID");
        assert_eq!(to_screaming_snake_case("user_id"), "USER_ID");
        assert_eq!(to_screaming_snake_case("HTTPResponse"), "HTTP_RESPONSE");
    }
}
