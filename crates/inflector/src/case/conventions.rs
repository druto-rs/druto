//! Full-stack convention helpers: tableize, classify, and foreign key generation.

use crate::{
    case::{pascal::to_pascal_case, snake::to_snake_case},
    grammar::{pluralize, singularize},
};

/// Converts a model name into a conventional database table name.
///
/// # Examples
/// ```
/// use inflector::case::tableize;
///
/// assert_eq!(tableize("UserProfile"), "user_profiles");
/// assert_eq!(tableize("Person"), "people");
/// assert_eq!(tableize("Category"), "categories");
/// ```
pub fn tableize(model_name: &str) -> String {
    let snake = to_snake_case(model_name);
    pluralize(&snake)
}

/// Converts a database table name into a conventional model class name.
///
/// # Examples
/// ```
/// use inflector::case::classify;
///
/// assert_eq!(classify("user_profiles"), "UserProfile");
/// assert_eq!(classify("people"), "Person");
/// assert_eq!(classify("categories"), "Category");
/// ```
pub fn classify(table_name: &str) -> String {
    let singular = singularize(table_name);
    to_pascal_case(&singular).into_owned()
}

/// Generates a foreign key column name from a model name with the default `_id` suffix.
///
/// # Examples
/// ```
/// use inflector::case::foreign_key;
///
/// assert_eq!(foreign_key("UserProfile"), "user_profile_id");
/// assert_eq!(foreign_key("Account"), "account_id");
/// ```
pub fn foreign_key(model_name: &str) -> String {
    foreign_key_with_suffix(model_name, "_id")
}

/// Generates a foreign key column name with a custom suffix.
pub fn foreign_key_with_suffix(model_name: &str, suffix: &str) -> String {
    let snake = to_snake_case(model_name);
    let mut result = String::with_capacity(snake.len() + suffix.len());
    result.push_str(&snake);
    result.push_str(suffix);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tableize() {
        assert_eq!(tableize("UserProfile"), "user_profiles");
        assert_eq!(tableize("Person"), "people");
        assert_eq!(tableize("Category"), "categories");
        assert_eq!(tableize("AdminUser"), "admin_users");
    }

    #[test]
    fn test_classify() {
        assert_eq!(classify("user_profiles"), "UserProfile");
        assert_eq!(classify("people"), "Person");
        assert_eq!(classify("categories"), "Category");
        assert_eq!(classify("admin_users"), "AdminUser");
    }

    #[test]
    fn test_foreign_key() {
        assert_eq!(foreign_key("UserProfile"), "user_profile_id");
        assert_eq!(foreign_key("Account"), "account_id");
        assert_eq!(
            foreign_key_with_suffix("UserProfile", "_pk"),
            "user_profile_pk"
        );
    }
}
