//! Casing style detection and preservation across grammatical inflections.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CasingStyle {
    AllUpper,
    TitleCase,
    LowerOrMixed,
}

/// Detects the dominant casing style of a word.
pub(crate) fn detect_casing_style(word: &str) -> CasingStyle {
    let mut alphabetic_chars = word.chars().filter(|c| c.is_alphabetic());
    let Some(first_char) = alphabetic_chars.next() else {
        return CasingStyle::LowerOrMixed;
    };

    if first_char.is_lowercase() {
        return CasingStyle::LowerOrMixed;
    }

    let mut found_lower = false;
    let mut found_additional_upper = false;

    for current_char in alphabetic_chars {
        if current_char.is_lowercase() {
            found_lower = true;
        } else {
            found_additional_upper = true;
        }
    }

    if !found_lower && found_additional_upper {
        CasingStyle::AllUpper
    } else {
        CasingStyle::TitleCase
    }
}

/// Reconstructs the transformed lowercase word with the given casing style.
pub(crate) fn apply_casing_style(transformed_lower: &str, style: CasingStyle) -> String {
    match style {
        CasingStyle::AllUpper => {
            let mut result = String::with_capacity(transformed_lower.len());
            for current_char in transformed_lower.chars() {
                for upper_char in current_char.to_uppercase() {
                    result.push(upper_char);
                }
            }
            result
        }
        CasingStyle::TitleCase => {
            let mut result = String::with_capacity(transformed_lower.len());
            for (char_idx, current_char) in transformed_lower.chars().enumerate() {
                if char_idx == 0 {
                    for upper_char in current_char.to_uppercase() {
                        result.push(upper_char);
                    }
                } else {
                    result.push(current_char);
                }
            }
            result
        }
        CasingStyle::LowerOrMixed => transformed_lower.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_and_apply_casing() {
        assert_eq!(detect_casing_style("user"), CasingStyle::LowerOrMixed);
        assert_eq!(detect_casing_style("User"), CasingStyle::TitleCase);
        assert_eq!(detect_casing_style("USER"), CasingStyle::AllUpper);

        assert_eq!(
            apply_casing_style("people", CasingStyle::LowerOrMixed),
            "people"
        );
        assert_eq!(
            apply_casing_style("people", CasingStyle::TitleCase),
            "People"
        );
        assert_eq!(
            apply_casing_style("people", CasingStyle::AllUpper),
            "PEOPLE"
        );
    }
}
