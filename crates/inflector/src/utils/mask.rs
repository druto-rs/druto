//! Sensitive string and credential masking utilities.

/// Masks a portion of a string with a chosen mask character.
///
/// Character indices operate safely on Unicode scalar values.
///
/// # Examples
/// ```
/// use inflector::utils::mask;
///
/// assert_eq!(mask("secret-token", '*', 3, Some(5)), "sec*****oken");
/// assert_eq!(mask("1234567812345678", '*', 0, Some(12)), "************5678");
/// ```
pub fn mask(input: &str, mask_char: char, start: usize, length: Option<usize>) -> String {
    let characters: Vec<char> = input.chars().collect();
    let total_characters = characters.len();

    if start >= total_characters {
        return input.to_string();
    }

    let mask_count = match length {
        Some(len) => len.min(total_characters - start),
        None => total_characters - start,
    };

    let mask_end = start + mask_count;
    let mut result = String::with_capacity(input.len());

    for (index, character) in characters.into_iter().enumerate() {
        if index >= start && index < mask_end {
            result.push(mask_char);
        } else {
            result.push(character);
        }
    }

    result
}

/// Masks the username portion of an email address for safe display.
///
/// # Examples
/// ```
/// use inflector::utils::mask_email;
///
/// assert_eq!(mask_email("johndoe@example.com", '*'), "j*****e@example.com");
/// assert_eq!(mask_email("a@example.com", '*'), "*@example.com");
/// assert_eq!(mask_email("ab@example.com", '*'), "a*@example.com");
/// ```
pub fn mask_email(email: &str, mask_char: char) -> String {
    let Some(at_index) = email.find('@') else {
        return mask(email, mask_char, 0, None);
    };

    let username = &email[..at_index];
    let domain = &email[at_index..];
    let username_chars: Vec<char> = username.chars().collect();
    let username_len = username_chars.len();

    let masked_username = match username_len {
        0 => String::new(),
        1 => mask_char.to_string(),
        2 => format!("{}{}", username_chars[0], mask_char),
        _ => {
            let first = username_chars[0];
            let last = username_chars[username_len - 1];
            let middle_count = username_len - 2;
            let mut result = String::with_capacity(username_len);
            result.push(first);
            for _ in 0..middle_count {
                result.push(mask_char);
            }
            result.push(last);
            result
        }
    };

    format!("{masked_username}{domain}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask() {
        assert_eq!(mask("secret-token", '*', 3, Some(5)), "sec*****oken");
        assert_eq!(
            mask("1234567812345678", '*', 0, Some(12)),
            "************5678"
        );
        assert_eq!(mask("short", '*', 10, Some(2)), "short");
        assert_eq!(mask("password", '*', 2, None), "pa******");
    }

    #[test]
    fn test_mask_email() {
        assert_eq!(
            mask_email("johndoe@example.com", '*'),
            "j*****e@example.com"
        );
        assert_eq!(mask_email("a@example.com", '*'), "*@example.com");
        assert_eq!(mask_email("ab@example.com", '*'), "a*@example.com");
        assert_eq!(mask_email("@example.com", '*'), "@example.com");
        assert_eq!(mask_email("not-an-email", '*'), "************");
    }
}
