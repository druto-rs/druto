//! String padding and boundary guards.

/// Computes remaining padding count and allocates a string with appropriate capacity.
/// Returns None if no padding is required.
fn prepare_padded_buffer(
    input: &str,
    target_len: usize,
    pad_char: char,
) -> Option<(usize, String)> {
    let current_len = input.chars().count();
    if current_len >= target_len {
        return None;
    }
    let pad_count = target_len - current_len;
    let buffer = String::with_capacity(input.len() + pad_count * pad_char.len_utf8());
    Some((pad_count, buffer))
}

fn push_repeated(buffer: &mut String, pad_char: char, count: usize) {
    buffer.extend(std::iter::repeat_n(pad_char, count));
}

/// Left-pads a string until it reaches the specified character length.
///
/// # Examples
/// ```
/// use inflector::utils::pad_start;
///
/// assert_eq!(pad_start("42", 5, '0'), "00042");
/// assert_eq!(pad_start("hello", 3, ' '), "hello");
/// ```
pub fn pad_start(input: &str, target_len: usize, pad_char: char) -> String {
    let Some((pad_count, mut result)) = prepare_padded_buffer(input, target_len, pad_char) else {
        return input.to_string();
    };
    push_repeated(&mut result, pad_char, pad_count);
    result.push_str(input);
    result
}

/// Right-pads a string until it reaches the specified character length.
///
/// # Examples
/// ```
/// use inflector::utils::pad_end;
///
/// assert_eq!(pad_end("42", 5, '0'), "42000");
/// assert_eq!(pad_end("hello", 3, ' '), "hello");
/// ```
pub fn pad_end(input: &str, target_len: usize, pad_char: char) -> String {
    let Some((pad_count, mut result)) = prepare_padded_buffer(input, target_len, pad_char) else {
        return input.to_string();
    };
    result.push_str(input);
    push_repeated(&mut result, pad_char, pad_count);
    result
}

/// Pads both sides of a string evenly until it reaches the specified character length.
///
/// # Examples
/// ```
/// use inflector::utils::pad_both;
///
/// assert_eq!(pad_both("title", 9, '-'), "--title--");
/// assert_eq!(pad_both("title", 10, '-'), "--title---");
/// ```
pub fn pad_both(input: &str, target_len: usize, pad_char: char) -> String {
    let Some((total_pad, mut result)) = prepare_padded_buffer(input, target_len, pad_char) else {
        return input.to_string();
    };
    let left_pad = total_pad / 2;
    let right_pad = total_pad - left_pad;
    push_repeated(&mut result, pad_char, left_pad);
    result.push_str(input);
    push_repeated(&mut result, pad_char, right_pad);
    result
}

/// Ensures a string starts with the given prefix.
///
/// # Examples
/// ```
/// use inflector::utils::start;
///
/// assert_eq!(start("path/to/resource", "/"), "/path/to/resource");
/// assert_eq!(start("/path/to/resource", "/"), "/path/to/resource");
/// ```
pub fn start(input: &str, prefix: &str) -> String {
    if input.starts_with(prefix) {
        input.to_string()
    } else {
        let mut result = String::with_capacity(prefix.len() + input.len());
        result.push_str(prefix);
        result.push_str(input);
        result
    }
}

/// Ensures a string ends with the given suffix.
///
/// # Examples
/// ```
/// use inflector::utils::finish;
///
/// assert_eq!(finish("https://example.com", "/"), "https://example.com/");
/// assert_eq!(finish("https://example.com/", "/"), "https://example.com/");
/// ```
pub fn finish(input: &str, suffix: &str) -> String {
    if input.ends_with(suffix) {
        input.to_string()
    } else {
        let mut result = String::with_capacity(input.len() + suffix.len());
        result.push_str(input);
        result.push_str(suffix);
        result
    }
}

/// Wraps a string with prefix and suffix strings.
///
/// # Examples
/// ```
/// use inflector::utils::wrap;
///
/// assert_eq!(wrap("value", "\"", "\""), "\"value\"");
/// assert_eq!(wrap("tag", "<", ">"), "<tag>");
/// ```
pub fn wrap(input: &str, before: &str, after: &str) -> String {
    let mut result = String::with_capacity(before.len() + input.len() + after.len());
    result.push_str(before);
    result.push_str(input);
    result.push_str(after);
    result
}

/// Removes enclosing prefix and suffix from a string if both are present.
///
/// # Examples
/// ```
/// use inflector::utils::unwrap;
///
/// assert_eq!(unwrap("\"value\"", "\"", "\""), "value");
/// assert_eq!(unwrap("<tag>", "<", ">"), "tag");
/// assert_eq!(unwrap("unwrapped", "<", ">"), "unwrapped");
/// ```
pub fn unwrap<'a>(input: &'a str, before: &str, after: &str) -> &'a str {
    if input.starts_with(before)
        && input.ends_with(after)
        && input.len() >= before.len() + after.len()
    {
        &input[before.len()..input.len() - after.len()]
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_padding() {
        assert_eq!(pad_start("42", 5, '0'), "00042");
        assert_eq!(pad_end("42", 5, '0'), "42000");
        assert_eq!(pad_both("title", 9, '-'), "--title--");
        assert_eq!(pad_both("title", 10, '-'), "--title---");
    }

    #[test]
    fn test_start_finish() {
        assert_eq!(start("path/to/resource", "/"), "/path/to/resource");
        assert_eq!(start("/path/to/resource", "/"), "/path/to/resource");
        assert_eq!(finish("https://example.com", "/"), "https://example.com/");
        assert_eq!(finish("https://example.com/", "/"), "https://example.com/");
    }

    #[test]
    fn test_wrap_unwrap() {
        assert_eq!(wrap("value", "\"", "\""), "\"value\"");
        assert_eq!(unwrap("\"value\"", "\"", "\""), "value");
        assert_eq!(unwrap("<tag>", "<", ">"), "tag");
        assert_eq!(unwrap("tag", "<", ">"), "tag");
    }
}
