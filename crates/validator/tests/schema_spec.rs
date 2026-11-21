//! Comprehensive tests for validator schemas covering primitive types, combinators, and rules.

use std::str::FromStr;

use serde::Deserialize;
use serde_json::json;
use validator::prelude::*;

#[derive(Debug, PartialEq, Eq, Deserialize)]
enum UserRole {
    Admin,
    Member,
    Guest,
}

impl FromStr for UserRole {
    type Err = String;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input.to_lowercase().as_str() {
            "admin" => Ok(Self::Admin),
            "member" => Ok(Self::Member),
            "guest" => Ok(Self::Guest),
            other => Err(format!("Unknown role: {other}")),
        }
    }
}

#[test]
fn test_string_schema_basic_success() {
    let s = string();
    let result = s.validate(&json!("hello world"));
    assert_eq!(result.unwrap(), "hello world");
}

#[test]
fn test_string_schema_type_mismatch() {
    let s = string();
    let err = s.validate(&json!(12345)).unwrap_err();
    assert_eq!(err.len(), 1);
    assert_eq!(err.issues()[0].rule, "string");
}

#[test]
fn test_string_trim_and_empty_to_null() {
    let s = string().trim(true).empty_string_as_null(true);
    let err = s.validate(&json!("   ")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "required");

    let valid = s.validate(&json!("   content   ")).unwrap();
    assert_eq!(valid, "content");
}

#[test]
fn test_string_min_max_exact_length() {
    let s = string().min_len(3).max_len(6);
    assert!(s.validate(&json!("abc")).is_ok());
    assert!(s.validate(&json!("abcdef")).is_ok());

    let err_short = s.validate(&json!("ab")).unwrap_err();
    assert_eq!(err_short.issues()[0].rule, "min_len");

    let err_long = s.validate(&json!("abcdefg")).unwrap_err();
    assert_eq!(err_long.issues()[0].rule, "max_len");

    let exact_schema = string().exact_len(4);
    assert!(exact_schema.validate(&json!("rust")).is_ok());
    let err_exact = exact_schema.validate(&json!("cpp")).unwrap_err();
    assert_eq!(err_exact.issues()[0].rule, "exact_len");
}

#[test]
fn test_string_email_validation() {
    let s = string().email();
    assert!(s.validate(&json!("contact@example.com")).is_ok());
    assert!(s.validate(&json!("user.name+tag@sub.domain.org")).is_ok());

    let err = s.validate(&json!("invalid-email")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "email");
}

#[test]
fn test_string_uuid_validation() {
    let s = string().uuid();
    assert!(
        s.validate(&json!("123e4567-e89b-12d3-a456-426614174000"))
            .is_ok()
    );
    assert!(
        s.validate(&json!("urn:uuid:123e4567-e89b-12d3-a456-426614174000"))
            .is_err()
    );

    let err = s.validate(&json!("not-a-uuid")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "uuid");
}

#[test]
fn test_string_url_validation() {
    let s = string().url();
    assert!(s.validate(&json!("https://example.com/api/v1")).is_ok());
    assert!(s.validate(&json!("http://localhost:8080/health")).is_ok());

    let err = s.validate(&json!("not_a_valid_url")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "url");
}

#[test]
fn test_string_ip_validation() {
    let ipv4_schema = string().ipv4();
    assert!(ipv4_schema.validate(&json!("192.168.1.1")).is_ok());
    assert!(ipv4_schema.validate(&json!("127.0.0.1")).is_ok());
    assert!(ipv4_schema.validate(&json!("999.999.999.999")).is_err());

    let ipv6_schema = string().ipv6();
    assert!(
        ipv6_schema
            .validate(&json!("2001:0db8:85a3:0000:0000:8a2e:0370:7334"))
            .is_ok()
    );
    assert!(ipv6_schema.validate(&json!("::1")).is_ok());
    assert!(ipv6_schema.validate(&json!("192.168.1.1")).is_err());

    let ip_schema = string().ip();
    assert!(ip_schema.validate(&json!("10.0.0.1")).is_ok());
    assert!(ip_schema.validate(&json!("fe80::1")).is_ok());
    assert!(ip_schema.validate(&json!("invalid_ip")).is_err());
}

#[test]
fn test_string_predicates_and_regex() {
    let s = string()
        .alpha()
        .starts_with("dr")
        .ends_with("to")
        .contains("ru");
    assert!(s.validate(&json!("druto")).is_ok());

    let err_alpha = s.validate(&json!("druto123")).unwrap_err();
    assert_eq!(err_alpha.issues()[0].rule, "alpha");

    let alphanumeric_schema = string().alphanumeric();
    assert!(alphanumeric_schema.validate(&json!("Token1234")).is_ok());
    assert!(alphanumeric_schema.validate(&json!("Token 1234")).is_err());

    let ascii_schema = string().ascii();
    assert!(ascii_schema.validate(&json!("regular ascii")).is_ok());
    assert!(ascii_schema.validate(&json!("unicode: \u{1F600}")).is_err());

    let regex_schema = string().regex(r"^[A-Z]{3}-\d{3}$");
    assert!(regex_schema.validate(&json!("ABC-123")).is_ok());
    assert!(regex_schema.validate(&json!("abc-123")).is_err());
}

#[test]
fn test_string_custom_rule_and_refine() {
    let s = string().refine_with(
        |val| val != "forbidden",
        "forbidden_word",
        "The value cannot be 'forbidden'.",
    );

    assert!(s.validate(&json!("allowed")).is_ok());
    let err = s.validate(&json!("forbidden")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "forbidden_word");
    assert_eq!(err.issues()[0].message, "The value cannot be 'forbidden'.");
}

#[test]
fn test_string_bail_behavior() {
    let exhaustive_schema = string()
        .min_len(5)
        .starts_with("prefix_")
        .ends_with("_suffix");
    let err_all = exhaustive_schema.validate(&json!("abc")).unwrap_err();
    assert_eq!(err_all.len(), 3);

    let bail_schema = string()
        .bail(true)
        .min_len(5)
        .starts_with("prefix_")
        .ends_with("_suffix");
    let err_bail = bail_schema.validate(&json!("abc")).unwrap_err();
    assert_eq!(err_bail.len(), 1);
    assert_eq!(err_bail.issues()[0].rule, "min_len");
}

#[test]
fn test_number_schema() {
    let n = number().min(10.0).max(100.0).positive();

    assert_eq!(n.validate(&json!(42.5)).unwrap(), 42.5);
    assert_eq!(n.validate(&json!("55.25")).unwrap(), 55.25);

    let err_min = n.validate(&json!(5.0)).unwrap_err();
    assert_eq!(err_min.issues()[0].rule, "min");

    let err_max = n.validate(&json!(150.0)).unwrap_err();
    assert_eq!(err_max.issues()[0].rule, "max");

    let range_schema = number().range(1.0, 10.0);
    assert!(range_schema.validate(&json!(5.0)).is_ok());
    assert!(range_schema.validate(&json!(0.5)).is_err());

    let negative_schema = number().negative();
    assert!(negative_schema.validate(&json!(-12.5)).is_ok());
    assert!(negative_schema.validate(&json!(12.5)).is_err());

    let int_check = number().int();
    assert!(int_check.validate(&json!(10.0)).is_ok());
    assert!(int_check.validate(&json!(10.5)).is_err());
}

#[test]
fn test_integer_schema() {
    let s = integer().min(1).max(10).multiple_of(2);

    assert_eq!(s.validate(&json!(4)).unwrap(), 4);
    assert_eq!(s.validate(&json!("8")).unwrap(), 8);

    let err_mult = s.validate(&json!(5)).unwrap_err();
    assert_eq!(err_mult.issues()[0].rule, "multiple_of");

    let err_invalid_str = s.validate(&json!("not_an_int")).unwrap_err();
    assert_eq!(err_invalid_str.issues()[0].rule, "number");

    let neg = integer().negative();
    assert!(neg.validate(&json!(-5)).is_ok());
    assert!(neg.validate(&json!(5)).is_err());
}

#[test]
fn test_boolean_schema() {
    let b = boolean();
    assert!(b.validate(&json!(true)).unwrap());
    assert!(!b.validate(&json!(false)).unwrap());

    assert!(b.validate(&json!("true")).unwrap());
    assert!(b.validate(&json!("1")).unwrap());
    assert!(b.validate(&json!("yes")).unwrap());
    assert!(b.validate(&json!("on")).unwrap());

    assert!(!b.validate(&json!("false")).unwrap());
    assert!(!b.validate(&json!("0")).unwrap());
    assert!(!b.validate(&json!("no")).unwrap());
    assert!(!b.validate(&json!("off")).unwrap());

    let err = b.validate(&json!("unrecognized")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "boolean");

    let terms = boolean().accepted();
    assert!(terms.validate(&json!(true)).is_ok());
    let err_not_accepted = terms.validate(&json!(false)).unwrap_err();
    assert_eq!(err_not_accepted.issues()[0].rule, "accepted");
}

#[test]
fn test_array_schema() {
    let s = array(string().email())
        .min_items(1)
        .max_items(3)
        .distinct()
        .not_empty();

    let valid_emails = json!(["alice@example.com", "bob@example.com"]);
    let validated = s.validate(&valid_emails).unwrap();
    assert_eq!(validated.len(), 2);

    let invalid_item = json!(["alice@example.com", "not_an_email"]);
    let err_item = s.validate(&invalid_item).unwrap_err();
    assert_eq!(err_item.issues()[0].field, "[1]");
    assert_eq!(err_item.issues()[0].rule, "email");

    let duplicates = json!(["alice@example.com", "alice@example.com"]);
    let err_dup = s.validate(&duplicates).unwrap_err();
    assert_eq!(err_dup.issues()[0].rule, "distinct");

    let empty_arr = json!([]);
    let err_empty = s.validate(&empty_arr).unwrap_err();
    assert!(
        err_empty
            .issues()
            .iter()
            .any(|issue| issue.rule == "not_empty" || issue.rule == "min_items")
    );

    let exact_s = array(integer()).exact_items(2);
    assert!(exact_s.validate(&json!([1, 2])).is_ok());
    assert!(exact_s.validate(&json!([1])).is_err());
}

#[test]
fn test_record_schema() {
    let s = record(integer().positive());
    let input = json!({
        "score_a": 100,
        "score_b": 250
    });

    let map = s.validate(&input).unwrap();
    assert_eq!(map.get("score_a"), Some(&100));
    assert_eq!(map.get("score_b"), Some(&250));

    // Test invalid value
    let invalid_val = json!({
        "valid_key": -10
    });
    let err_val = s.validate(&invalid_val).unwrap_err();
    assert_eq!(err_val.issues()[0].rule, "positive");
    assert_eq!(err_val.issues()[0].field, "valid_key");
}

#[test]
fn test_enum_schema() {
    let s = r#enum::<UserRole>().variants(&["admin", "member", "guest"]);
    assert_eq!(s.validate(&json!("admin")).unwrap(), UserRole::Admin);
    assert_eq!(s.validate(&json!("MEMBER")).unwrap(), UserRole::Member);
    assert_eq!(s.validate(&json!("guest")).unwrap(), UserRole::Guest);

    let err = s.validate(&json!("superuser")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "enum");
}

#[test]
fn test_optional_nullable_and_default_schema() {
    let opt = string().optional();
    assert_eq!(
        opt.validate(&json!("hello")).unwrap(),
        Some("hello".to_string())
    );
    assert_eq!(opt.validate(&json!(null)).unwrap(), None);

    let nullable_s = integer().nullable();
    assert_eq!(nullable_s.validate(&json!(42)).unwrap(), Some(42));
    assert_eq!(nullable_s.validate(&json!(null)).unwrap(), None);

    let def = string().default_value("anonymous".to_string());
    assert_eq!(def.validate(&json!("alice")).unwrap(), "alice");
    assert_eq!(def.validate(&json!(null)).unwrap(), "anonymous");
}

#[test]
fn test_tuple2_schema() {
    let s = tuple2(string().min_len(2), integer().positive());
    let (name, score) = s.validate(&json!(["druto", 99])).unwrap();
    assert_eq!(name, "druto");
    assert_eq!(score, 99);

    let err_len = s.validate(&json!(["druto"])).unwrap_err();
    assert_eq!(err_len.issues()[0].rule, "exact_items");

    let err_elements = s.validate(&json!(["d", -5])).unwrap_err();
    assert_eq!(err_elements.len(), 2);
    assert_eq!(err_elements.issues()[0].field, "[0]");
    assert_eq!(err_elements.issues()[1].field, "[1]");
}

#[test]
fn test_union2_schema() {
    let s = union2(
        integer().map(|n| format!("number: {n}")),
        string().map(|s| format!("text: {s}")),
    );

    assert_eq!(s.validate(&json!(100)).unwrap(), "number: 100");
    assert_eq!(s.validate(&json!("hello")).unwrap(), "text: hello");

    let err = s.validate(&json!(true)).unwrap_err();
    assert!(!err.is_empty());
}

#[test]
fn test_transform_and_map() {
    let s = string().transform(|val| val.parse::<i32>().map_err(|err| err.to_string()));

    assert_eq!(s.validate(&json!("123")).unwrap(), 123);
    let err = s.validate(&json!("abc")).unwrap_err();
    assert_eq!(err.issues()[0].rule, "transform");
}
