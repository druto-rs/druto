//! Sentence case (`Sentence case`) conversions.

use std::borrow::Cow;

use super::split_words;

/// Checks if the input slice is already in valid `Sentence case` without allocating.
///
/// A valid `Sentence case` string begins with an uppercase character followed by
/// lowercase characters or digits, with words separated by single spaces.
pub fn is_sentence_case(input: &str) -> bool {
    let mut characters = input.chars();
    let Some(first_char) = characters.next() else {
        return false;
    };

    if !first_char.is_uppercase() || input.ends_with(' ') {
        return false;
    }

    let mut previous_was_space = false;
    for current_char in characters {
        if current_char == ' ' {
            if previous_was_space {
                return false;
            }
            previous_was_space = true;
        } else if current_char.is_lowercase() || current_char.is_numeric() {
            previous_was_space = false;
        } else {
            return false;
        }
    }

    !previous_was_space
}

/// Converts a string slice to `Sentence case`.
///
/// Returns `Cow::Borrowed` with zero allocations if the input already conforms to `Sentence case`.
pub fn to_sentence_case(input: &str) -> Cow<'_, str> {
    if is_sentence_case(input) {
        return Cow::Borrowed(input);
    }

    let words = split_words(input);
    if words.is_empty() {
        return Cow::Borrowed("");
    }

    let total_separator_bytes = words.len().saturating_sub(1);
    let estimated_capacity = input.len() + total_separator_bytes;
    let mut result = String::with_capacity(estimated_capacity);

    for (index, word) in words.iter().enumerate() {
        if index > 0 {
            result.push(' ');
        }
        for (char_idx, current_char) in word.chars().enumerate() {
            if index == 0 && char_idx == 0 {
                for upper_char in current_char.to_uppercase() {
                    result.push(upper_char);
                }
            } else {
                for lower_char in current_char.to_lowercase() {
                    result.push(lower_char);
                }
            }
        }
    }

    Cow::Owned(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_sentence_case() {
        assert!(is_sentence_case("User profile"));
        assert!(is_sentence_case("Foo bar baz"));
        assert!(is_sentence_case("Http response"));

        assert!(!is_sentence_case(""));
        assert!(!is_sentence_case("user profile"));
        assert!(!is_sentence_case("User Profile"));
        assert!(!is_sentence_case("User profile "));
        assert!(!is_sentence_case("User  profile"));
    }

    #[test]
    fn test_to_sentence_case_borrowed() {
        let input = "User profile";
        let result = to_sentence_case(input);
        assert!(matches!(result, Cow::Borrowed(_)));
        assert_eq!(result, "User profile");
    }

    #[test]
    fn test_to_sentence_case_conversions() {
        assert_eq!(to_sentence_case("user_profile"), "User profile");
        assert_eq!(to_sentence_case("userProfile"), "User profile");
        assert_eq!(to_sentence_case("HTTPResponse"), "Http response");
        assert_eq!(to_sentence_case(""), "");
    }
}
