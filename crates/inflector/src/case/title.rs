//! Title Case (`Title Case`) conversions.

use std::borrow::Cow;

use super::{is_delimited_title, join_words_title, split_words};

/// Checks if the input slice is already in valid `Title Case` without allocating.
///
/// A valid `Title Case` string consists of non-empty words separated by single spaces,
/// where each word begins with an uppercase character followed by lowercase characters or digits.
#[inline]
pub fn is_title_case(input: &str) -> bool {
    is_delimited_title(input, ' ')
}

/// Converts a string slice to `Title Case`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `Title Case`.
pub fn to_title_case(input: &str) -> Cow<'_, str> {
    if is_title_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    Cow::Owned(join_words_title(&words, ' ', input.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_title_case() {
        assert!(is_title_case("User Profile"));
        assert!(is_title_case("Foo Bar Baz"));
        assert!(is_title_case("Http Response"));

        assert!(!is_title_case(""));
        assert!(!is_title_case("user profile"));
        assert!(!is_title_case("User profile"));
        assert!(!is_title_case(" User Profile"));
        assert!(!is_title_case("User Profile "));
        assert!(!is_title_case("User  Profile"));
        assert!(!is_title_case("USER PROFILE"));
    }

    #[test]
    fn test_to_title_case_borrowed() {
        let input = "User Profile";
        let result = to_title_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "User Profile");
    }

    #[test]
    fn test_to_title_case_conversions() {
        assert_eq!(to_title_case("user_profile"), "User Profile");
        assert_eq!(to_title_case("userProfile"), "User Profile");
        assert_eq!(to_title_case("HTTPResponse"), "Http Response");
        assert_eq!(to_title_case(""), "");
    }
}
