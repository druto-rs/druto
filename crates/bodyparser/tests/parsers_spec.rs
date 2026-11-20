//! Unit tests for JSON, Form, and Text content-type parsers.

use bodyparser::{
    BodyParserError, PayloadLimits, parse_form, parse_form_value, parse_json, parse_json_value,
    parse_text,
};
use serde::Deserialize;

#[derive(Debug, PartialEq, Deserialize)]
struct User {
    name: String,
    age: u32,
}

#[test]
fn test_parse_json_valid() {
    let payload = br#"{"name":"Alice","age":28}"#;
    let limits = PayloadLimits::default();

    let user: User = parse_json(payload, &limits).unwrap();
    assert_eq!(
        user,
        User {
            name: "Alice".to_string(),
            age: 28,
        }
    );

    let value = parse_json_value(payload, &limits).unwrap();
    assert_eq!(value["name"], "Alice");
    assert_eq!(value["age"], 28);
}

#[test]
fn test_parse_json_depth_limit_exceeded() {
    let payload = br#"{"a":{"b":{"c":{"d":1}}}}"#;
    let limits = PayloadLimits::builder().max_depth(3).build();

    let err = parse_json_value(payload, &limits).unwrap_err();
    assert_eq!(err, BodyParserError::NestingDepthExceeded { max_depth: 3 });

    // Test array nesting depth limit exceeded
    let array_payload = br#"[[[[1]]]]"#;
    let err_array = parse_json_value(array_payload, &limits).unwrap_err();
    assert_eq!(
        err_array,
        BodyParserError::NestingDepthExceeded { max_depth: 3 }
    );
}

#[test]
fn test_parse_json_size_limit_exceeded() {
    let payload = br#"{"name":"Alice"}"#;
    let limits = PayloadLimits::builder().max_body_size(5).build();

    let err = parse_json_value(payload, &limits).unwrap_err();
    assert_eq!(
        err,
        BodyParserError::PayloadTooLarge {
            limit: 5,
            received: payload.len(),
        }
    );
}

#[test]
fn test_parse_json_syntax_error() {
    let payload = b"{invalid json";
    let limits = PayloadLimits::default();

    let err = parse_json_value(payload, &limits).unwrap_err();
    assert!(matches!(err, BodyParserError::JsonError(_)));
}

#[test]
fn test_parse_form_valid() {
    let payload = b"name=Bob&age=35";
    let limits = PayloadLimits::default();

    let user: User = parse_form(payload, &limits).unwrap();
    assert_eq!(
        user,
        User {
            name: "Bob".to_string(),
            age: 35,
        }
    );

    let value = parse_form_value(payload, &limits).unwrap();
    assert_eq!(value.get("name").unwrap().as_scalar(), Some("Bob"));
    assert_eq!(value.get("age").unwrap().as_scalar(), Some("35"));
}

#[test]
fn test_parse_form_nested() {
    let payload = b"user[name]=Charlie&user[age]=42";
    let limits = PayloadLimits::default();

    let value = parse_form_value(payload, &limits).unwrap();
    let user = value.get("user").unwrap();
    assert_eq!(user.get("name").unwrap().as_scalar(), Some("Charlie"));
    assert_eq!(user.get("age").unwrap().as_scalar(), Some("42"));
}

#[test]
fn test_parse_text_valid() {
    let payload = b"Hello, World!";
    let limits = PayloadLimits::default();

    let text = parse_text(payload, &limits).unwrap();
    assert_eq!(text, "Hello, World!");
}

#[test]
fn test_parse_text_invalid_utf8() {
    let payload = &[0xFF, 0xFE, 0xFD];
    let limits = PayloadLimits::default();

    let err = parse_text(payload, &limits).unwrap_err();
    assert!(matches!(err, BodyParserError::Utf8Error(_)));
}
