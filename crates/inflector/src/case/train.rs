//! Train case (`Train-Case` / `Header-Case`) conversions.

use std::borrow::Cow;

use super::split_words;

/// Checks if the input slice is already in valid `Train-Case` without allocating.
///
/// A valid `Train-Case` string consists of non-empty words separated by single hyphens,
/// where each word begins with an uppercase character followed by lowercase characters or digits.
pub fn is_train_case(input: &str) -> bool {
    super::is_delimited_title(input, '-')
}

/// Converts a string slice to `Train-Case`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `Train-Case`.
pub fn to_train_case(input: &str) -> Cow<'_, str> {
    if is_train_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(super::join_words_title(&words, '-', input.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_train_case() {
        assert!(is_train_case("User-Profile"));
        assert!(is_train_case("Content-Type"));
        assert!(is_train_case("Http-Response"));

        assert!(!is_train_case(""));
        assert!(!is_train_case("user-profile"));
        assert!(!is_train_case("User-profile"));
        assert!(!is_train_case("-User-Profile"));
        assert!(!is_train_case("User-Profile-"));
        assert!(!is_train_case("User--Profile"));
        assert!(!is_train_case("USER-PROFILE"));
    }

    #[test]
    fn test_to_train_case_borrowed() {
        let input = "Content-Type";
        let result = to_train_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "Content-Type");
    }

    #[test]
    fn test_to_train_case_conversions() {
        assert_eq!(to_train_case("content_type"), "Content-Type");
        assert_eq!(to_train_case("userProfile"), "User-Profile");
        assert_eq!(to_train_case("HTTPResponse"), "Http-Response");
        assert_eq!(to_train_case(""), "");
    }
}
