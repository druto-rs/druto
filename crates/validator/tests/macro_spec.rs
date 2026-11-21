//! Tests for declarative `schema!` and `request_schema!` macro DSLs.

#![cfg(feature = "macros")]

use serde_json::json;
use validator::prelude::*;

#[test]
fn test_declarative_schema_macro_simple() {
    let user_schema = schema! {
        name: string().min_len(3),
        age: integer().positive(),
    };

    let valid_input = json!({
        "name": "Druto",
        "age": 2
    });
    let result = user_schema.validate(&valid_input).unwrap();
    assert_eq!(result.get("name").and_then(|v| v.as_str()), Some("Druto"));
    assert_eq!(result.get("age").and_then(|v| v.as_i64()), Some(2));

    let invalid_input = json!({
        "name": "ab",
        "age": -1
    });
    let err = user_schema.validate(&invalid_input).unwrap_err();
    assert_eq!(err.len(), 2);
}

#[test]
fn test_declarative_schema_macro_optional_field() {
    let post_schema = schema! {
        title: string().min_len(5),
        subtitle?: string().max_len(100),
    };

    let with_optional = json!({
        "title": "Rust Web",
        "subtitle": "Building robust servers"
    });
    assert!(post_schema.validate(&with_optional).is_ok());

    let without_optional = json!({
        "title": "Rust Web"
    });
    assert!(post_schema.validate(&without_optional).is_ok());
}

#[test]
fn test_declarative_request_schema_macro() {
    let article_request = request_schema! {
        params: schema! {
            id: integer().positive(),
        },
        query: schema! {
            draft?: boolean(),
        },
        body: schema! {
            title: string().min_len(5),
        },
    };

    let valid_parts = RequestParts {
        params: json!({ "id": 10 }),
        query: json!({ "draft": true }),
        headers: json!({}),
        cookies: json!({}),
        body: json!({ "title": "Declarative Web APIs" }),
    };

    let validated = article_request.validate(&valid_parts).unwrap();
    assert!(validated.params.is_some());
    assert!(validated.query.is_some());
    assert!(validated.body.is_some());
}
