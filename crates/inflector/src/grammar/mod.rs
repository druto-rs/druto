//! Grammatical inflection engine for English pluralization and singularization.

use std::{
    collections::{HashMap, HashSet},
    sync::{LazyLock, RwLock},
};

mod case_preserver;
mod rules;
pub mod uncountables;

use case_preserver::{apply_casing_style, detect_casing_style};

/// Thread-safe global default inflection engine instance.
static GLOBAL_INFLECTOR: LazyLock<RwLock<Inflector>> =
    LazyLock::new(|| RwLock::new(Inflector::new()));

/// English grammatical inflection engine with custom rule support.
#[derive(Debug, Clone)]
pub struct Inflector {
    custom_uncountables: HashSet<String>,
    custom_irregulars_singular_to_plural: HashMap<String, String>,
    custom_irregulars_plural_to_singular: HashMap<String, String>,
    #[cfg(feature = "regex")]
    custom_plural_rules: Vec<(regex::Regex, String)>,
    #[cfg(feature = "regex")]
    custom_singular_rules: Vec<(regex::Regex, String)>,
}

impl Default for Inflector {
    fn default() -> Self {
        Self::new()
    }
}

impl Inflector {
    /// Creates a new `Inflector` with standard English grammar rules.
    pub fn new() -> Self {
        Self {
            custom_uncountables: HashSet::new(),
            custom_irregulars_singular_to_plural: HashMap::new(),
            custom_irregulars_plural_to_singular: HashMap::new(),
            #[cfg(feature = "regex")]
            custom_plural_rules: Vec::new(),
            #[cfg(feature = "regex")]
            custom_singular_rules: Vec::new(),
        }
    }

    /// Registers a custom word that should never be inflected.
    pub fn add_uncountable(&mut self, word: &str) {
        self.custom_uncountables.insert(word.to_lowercase());
    }

    /// Registers a custom irregular singular/plural pair.
    pub fn add_irregular(&mut self, singular: &str, plural: &str) {
        let singular_lower = singular.to_lowercase();
        let plural_lower = plural.to_lowercase();
        self.custom_irregulars_singular_to_plural
            .insert(singular_lower.clone(), plural_lower.clone());
        self.custom_irregulars_plural_to_singular
            .insert(plural_lower, singular_lower);
    }

    /// Registers a custom regular expression rule for pluralization.
    #[cfg(feature = "regex")]
    pub fn add_plural_rule(
        &mut self,
        pattern: &str,
        replacement: &str,
    ) -> Result<(), regex::Error> {
        let compiled = regex::Regex::new(pattern)?;
        self.custom_plural_rules
            .push((compiled, replacement.to_string()));
        Ok(())
    }

    /// Registers a custom regular expression rule for singularization.
    #[cfg(feature = "regex")]
    pub fn add_singular_rule(
        &mut self,
        pattern: &str,
        replacement: &str,
    ) -> Result<(), regex::Error> {
        let compiled = regex::Regex::new(pattern)?;
        self.custom_singular_rules
            .push((compiled, replacement.to_string()));
        Ok(())
    }

    /// Converts a word or identifier into its plural form while preserving letter casing.
    pub fn pluralize(&self, word: &str) -> String {
        if word.is_empty() {
            return String::new();
        }

        // Handle delimited compound strings (e.g., "user_profile", "user-profile", "user profile")
        if let Some(separator_index) = word.rfind(['_', '-', ' ']) {
            let prefix = &word[..=separator_index];
            let last_word = &word[separator_index + 1..];
            if !last_word.is_empty() {
                let inflected_last = self.pluralize_single_word(last_word);
                let mut result = String::with_capacity(prefix.len() + inflected_last.len());
                result.push_str(prefix);
                result.push_str(&inflected_last);
                return result;
            }
        }

        // Handle camelCase or PascalCase compound strings (e.g., "userProfile", "UserProfile")
        let words = crate::case::split_words(word);
        if words.len() > 1 && !word.contains(['_', '-', ' ']) {
            let last_token = words[words.len() - 1];
            let last_offset = word.len() - last_token.len();
            let prefix = &word[..last_offset];
            let inflected_last = self.pluralize_single_word(last_token);
            let mut result = String::with_capacity(prefix.len() + inflected_last.len());
            result.push_str(prefix);
            result.push_str(&inflected_last);
            return result;
        }

        self.pluralize_single_word(word)
    }

    /// Converts a word or identifier into its singular form while preserving letter casing.
    pub fn singularize(&self, word: &str) -> String {
        if word.is_empty() {
            return String::new();
        }

        // Handle delimited compound strings
        if let Some(separator_index) = word.rfind(['_', '-', ' ']) {
            let prefix = &word[..=separator_index];
            let last_word = &word[separator_index + 1..];
            if !last_word.is_empty() {
                let inflected_last = self.singularize_single_word(last_word);
                let mut result = String::with_capacity(prefix.len() + inflected_last.len());
                result.push_str(prefix);
                result.push_str(&inflected_last);
                return result;
            }
        }

        // Handle camelCase or PascalCase compound strings
        let words = crate::case::split_words(word);
        if words.len() > 1 && !word.contains(['_', '-', ' ']) {
            let last_token = words[words.len() - 1];
            let last_offset = word.len() - last_token.len();
            let prefix = &word[..last_offset];
            let inflected_last = self.singularize_single_word(last_token);
            let mut result = String::with_capacity(prefix.len() + inflected_last.len());
            result.push_str(prefix);
            result.push_str(&inflected_last);
            return result;
        }

        self.singularize_single_word(word)
    }

    /// Inflects the word based on count: singular if count is 1, plural otherwise.
    pub fn pluralize_with_count(&self, word: &str, count: usize) -> String {
        if count == 1 {
            self.singularize(word)
        } else {
            self.pluralize(word)
        }
    }

    /// Inflects the word and prefixes the count: e.g. "1 user", "2 users".
    pub fn pluralize_with_count_inclusive(&self, word: &str, count: usize) -> String {
        let inflected = self.pluralize_with_count(word, count);
        format!("{count} {inflected}")
    }

    /// Determines if a word is in plural form.
    pub fn is_plural(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();
        if self.custom_uncountables.contains(&word_lower)
            || uncountables::is_uncountable(&word_lower)
        {
            return true;
        }
        if self
            .custom_irregulars_plural_to_singular
            .contains_key(&word_lower)
            || rules::find_irregular_singular(&word_lower).is_some()
        {
            return true;
        }
        if self
            .custom_irregulars_singular_to_plural
            .contains_key(&word_lower)
            || rules::find_irregular_plural(&word_lower).is_some()
        {
            return false;
        }
        self.singularize(word) != word
    }

    /// Determines if a word is in singular form.
    pub fn is_singular(&self, word: &str) -> bool {
        let word_lower = word.to_lowercase();
        if self.custom_uncountables.contains(&word_lower)
            || uncountables::is_uncountable(&word_lower)
        {
            return true;
        }
        if self
            .custom_irregulars_singular_to_plural
            .contains_key(&word_lower)
            || rules::find_irregular_plural(&word_lower).is_some()
        {
            return true;
        }
        if self
            .custom_irregulars_plural_to_singular
            .contains_key(&word_lower)
            || rules::find_irregular_singular(&word_lower).is_some()
        {
            return false;
        }
        self.singularize(word) == word
    }

    fn pluralize_single_word(&self, word: &str) -> String {
        let casing_style = detect_casing_style(word);
        let word_lower = word.to_lowercase();

        if self.custom_uncountables.contains(&word_lower)
            || uncountables::is_uncountable(&word_lower)
        {
            return apply_casing_style(&word_lower, casing_style);
        }

        if let Some(plural) = self.custom_irregulars_singular_to_plural.get(&word_lower) {
            return apply_casing_style(plural, casing_style);
        }

        if let Some(plural) = rules::find_irregular_plural(&word_lower) {
            return apply_casing_style(plural, casing_style);
        }

        #[cfg(feature = "regex")]
        for (pattern, replacement) in self.custom_plural_rules.iter().rev() {
            if pattern.is_match(&word_lower) {
                let replaced = pattern.replace(&word_lower, replacement.as_str());
                return apply_casing_style(&replaced, casing_style);
            }
        }

        let plural_lower = rules::pluralize_suffix(&word_lower);
        apply_casing_style(&plural_lower, casing_style)
    }

    fn singularize_single_word(&self, word: &str) -> String {
        let casing_style = detect_casing_style(word);
        let word_lower = word.to_lowercase();

        if self.custom_uncountables.contains(&word_lower)
            || uncountables::is_uncountable(&word_lower)
        {
            return apply_casing_style(&word_lower, casing_style);
        }

        if let Some(singular) = self.custom_irregulars_plural_to_singular.get(&word_lower) {
            return apply_casing_style(singular, casing_style);
        }

        if let Some(singular) = rules::find_irregular_singular(&word_lower) {
            return apply_casing_style(singular, casing_style);
        }

        #[cfg(feature = "regex")]
        for (pattern, replacement) in self.custom_singular_rules.iter().rev() {
            if pattern.is_match(&word_lower) {
                let replaced = pattern.replace(&word_lower, replacement.as_str());
                return apply_casing_style(&replaced, casing_style);
            }
        }

        let singular_lower = rules::singularize_suffix(&word_lower);
        apply_casing_style(&singular_lower, casing_style)
    }
}

/// Global helper: Pluralizes a word or identifier using the default engine.
pub fn pluralize(word: &str) -> String {
    GLOBAL_INFLECTOR
        .read()
        .expect("inflector lock poisoned")
        .pluralize(word)
}

/// Global helper: Singularizes a word or identifier using the default engine.
pub fn singularize(word: &str) -> String {
    GLOBAL_INFLECTOR
        .read()
        .expect("inflector lock poisoned")
        .singularize(word)
}

/// Global helper: Inflects based on count using the default engine.
pub fn pluralize_with_count(word: &str, count: usize) -> String {
    GLOBAL_INFLECTOR
        .read()
        .expect("inflector lock poisoned")
        .pluralize_with_count(word, count)
}

/// Global helper: Inflects with count prefix using the default engine.
pub fn pluralize_with_count_inclusive(word: &str, count: usize) -> String {
    GLOBAL_INFLECTOR
        .read()
        .expect("inflector lock poisoned")
        .pluralize_with_count_inclusive(word, count)
}

/// Global helper: Checks if a word is in plural form.
pub fn is_plural(word: &str) -> bool {
    GLOBAL_INFLECTOR
        .read()
        .expect("inflector lock poisoned")
        .is_plural(word)
}

/// Global helper: Checks if a word is in singular form.
pub fn is_singular(word: &str) -> bool {
    GLOBAL_INFLECTOR
        .read()
        .expect("inflector lock poisoned")
        .is_singular(word)
}

/// Registers an uncountable word in the global engine.
pub fn add_uncountable(word: &str) {
    GLOBAL_INFLECTOR
        .write()
        .expect("inflector lock poisoned")
        .add_uncountable(word);
}

/// Registers an irregular pair in the global engine.
pub fn add_irregular(singular: &str, plural: &str) {
    GLOBAL_INFLECTOR
        .write()
        .expect("inflector lock poisoned")
        .add_irregular(singular, plural);
}

/// Registers a custom pluralization regex rule in the global engine.
#[cfg(feature = "regex")]
pub fn add_plural_rule(pattern: &str, replacement: &str) -> Result<(), regex::Error> {
    GLOBAL_INFLECTOR
        .write()
        .expect("inflector lock poisoned")
        .add_plural_rule(pattern, replacement)
}

/// Registers a custom singularization regex rule in the global engine.
#[cfg(feature = "regex")]
pub fn add_singular_rule(pattern: &str, replacement: &str) -> Result<(), regex::Error> {
    GLOBAL_INFLECTOR
        .write()
        .expect("inflector lock poisoned")
        .add_singular_rule(pattern, replacement)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regular_pluralization_and_singularization() {
        assert_eq!(pluralize("user"), "users");
        assert_eq!(singularize("users"), "user");

        assert_eq!(pluralize("category"), "categories");
        assert_eq!(singularize("categories"), "category");

        assert_eq!(pluralize("box"), "boxes");
        assert_eq!(singularize("boxes"), "box");

        assert_eq!(pluralize("knife"), "knives");
        assert_eq!(singularize("knives"), "knife");

        assert_eq!(pluralize("leaf"), "leaves");
        assert_eq!(singularize("leaves"), "leaf");

        assert_eq!(pluralize("hero"), "heroes");
        assert_eq!(singularize("heroes"), "hero");

        assert_eq!(pluralize("status"), "statuses");
        assert_eq!(singularize("statuses"), "status");
    }

    #[test]
    fn test_irregular_pluralization_and_singularization() {
        assert_eq!(pluralize("person"), "people");
        assert_eq!(singularize("people"), "person");

        assert_eq!(pluralize("child"), "children");
        assert_eq!(singularize("children"), "child");

        assert_eq!(pluralize("ox"), "oxen");
        assert_eq!(singularize("oxen"), "ox");

        assert_eq!(pluralize("mouse"), "mice");
        assert_eq!(singularize("mice"), "mouse");

        assert_eq!(pluralize("matrix"), "matrices");
        assert_eq!(singularize("matrices"), "matrix");

        assert_eq!(pluralize("crisis"), "crises");
        assert_eq!(singularize("crises"), "crisis");
    }

    #[test]
    fn test_uncountable_words() {
        assert_eq!(pluralize("money"), "money");
        assert_eq!(singularize("money"), "money");

        assert_eq!(pluralize("information"), "information");
        assert_eq!(singularize("information"), "information");

        assert_eq!(pluralize("rice"), "rice");
        assert_eq!(singularize("rice"), "rice");

        assert_eq!(pluralize("equipment"), "equipment");
        assert_eq!(singularize("equipment"), "equipment");
    }

    #[test]
    fn test_casing_preservation() {
        assert_eq!(pluralize("User"), "Users");
        assert_eq!(singularize("Users"), "User");

        assert_eq!(pluralize("USER"), "USERS");
        assert_eq!(singularize("USERS"), "USER");

        assert_eq!(pluralize("Person"), "People");
        assert_eq!(singularize("People"), "Person");

        assert_eq!(pluralize("PERSON"), "PEOPLE");
        assert_eq!(singularize("PEOPLE"), "PERSON");
    }

    #[test]
    fn test_compound_identifiers() {
        assert_eq!(pluralize("user_profile"), "user_profiles");
        assert_eq!(singularize("user_profiles"), "user_profile");

        assert_eq!(pluralize("user-profile"), "user-profiles");
        assert_eq!(singularize("user-profiles"), "user-profile");

        assert_eq!(pluralize("userProfile"), "userProfiles");
        assert_eq!(singularize("userProfiles"), "userProfile");

        assert_eq!(pluralize("UserProfile"), "UserProfiles");
        assert_eq!(singularize("UserProfiles"), "UserProfile");

        assert_eq!(pluralize("user_"), "user_s");
        assert_eq!(singularize("user_"), "user_");
    }

    #[test]
    fn test_pluralize_with_count() {
        assert_eq!(pluralize_with_count("box", 1), "box");
        assert_eq!(pluralize_with_count("box", 2), "boxes");
        assert_eq!(pluralize_with_count_inclusive("box", 1), "1 box");
        assert_eq!(pluralize_with_count_inclusive("box", 5), "5 boxes");
    }

    #[test]
    fn test_custom_rule_registration() {
        let mut engine = Inflector::new();
        engine.add_irregular("pokemon", "pokemon");
        assert_eq!(engine.pluralize("pokemon"), "pokemon");
        assert_eq!(engine.singularize("pokemon"), "pokemon");

        engine.add_uncountable("jargon");
        assert_eq!(engine.pluralize("jargon"), "jargon");

        #[cfg(feature = "regex")]
        {
            engine
                .add_plural_rule(r"([a-zA-Z0-9_]*)globperson$", "${1}globs")
                .unwrap();
            engine
                .add_singular_rule(r"([a-zA-Z0-9_]*)globs$", "${1}globperson")
                .unwrap();

            assert_eq!(engine.pluralize("superglobperson"), "superglobs");
            assert_eq!(engine.pluralize("user"), "users");

            assert_eq!(engine.singularize("superglobs"), "superglobperson");
            assert_eq!(engine.singularize("users"), "user");
        }
    }
}
