//! Tests for RequestSchema, multi-source HTTP validation, namespacing, and typed extractions.

use serde::Deserialize;
use serde_json::json;
use validator::prelude::*;

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct UserParams {
    id: i64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct SearchQuery {
    query: String,
    page: i64,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct CreateArticleBody {
    title: String,
    content: String,
}

#[test]
fn test_request_schema_all_parts_valid() {
    let req_schema = request()
        .params(object().field("id", integer().positive()))
        .query(
            object()
                .field("query", string().min_len(2))
                .field("page", integer().positive()),
        )
        .body(
            object()
                .field("title", string().min_len(5))
                .field("content", string().min_len(10)),
        );

    let parts = RequestParts {
        params: json!({ "id": 42 }),
        query: json!({ "query": "rust", "page": 1 }),
        headers: json!({ "accept": "application/json" }),
        cookies: json!({ "session_id": "sess_123" }),
        body: json!({
            "title": "Clean Web Framework",
            "content": "A deep module architecture written in idiomatic Rust."
        }),
    };

    let validated = req_schema.validate(&parts).unwrap();

    let params: UserParams = validated.decode_params().unwrap();
    assert_eq!(params.id, 42);

    let query: SearchQuery = validated.decode_query().unwrap();
    assert_eq!(query.query, "rust");
    assert_eq!(query.page, 1);

    let body: CreateArticleBody = validated.decode_body().unwrap();
    assert_eq!(body.title, "Clean Web Framework");
}

#[test]
fn test_request_schema_namespaced_errors() {
    let req_schema = request()
        .params(object().field("id", integer().positive()))
        .query(object().field("page", integer().positive()))
        .body(object().field("title", string().min_len(10)));

    let invalid_parts = RequestParts {
        params: json!({ "id": -5 }),
        query: json!({ "page": 0 }),
        headers: json!({}),
        cookies: json!({}),
        body: json!({ "title": "short" }),
    };

    let err = req_schema.validate(&invalid_parts).unwrap_err();
    assert_eq!(err.len(), 3);

    let field_paths: Vec<&str> = err.issues().iter().map(|i| i.field.as_str()).collect();
    assert!(field_paths.contains(&"params.id"));
    assert!(field_paths.contains(&"query.page"));
    assert!(field_paths.contains(&"body.title"));
}

#[test]
fn test_request_schema_cookies_and_headers() {
    let req_schema = request()
        .headers(object().field("authorization", string().starts_with("Bearer ")))
        .cookies(object().field("session_token", string().uuid()));

    let valid_parts = RequestParts {
        params: json!({}),
        query: json!({}),
        headers: json!({ "authorization": "Bearer token12345" }),
        cookies: json!({ "session_token": "123e4567-e89b-12d3-a456-426614174000" }),
        body: json!({}),
    };

    let result = req_schema.validate(&valid_parts);
    assert!(result.is_ok());

    let invalid_parts = RequestParts {
        params: json!({}),
        query: json!({}),
        headers: json!({ "authorization": "Basic user:pass" }),
        cookies: json!({ "session_token": "not_a_uuid" }),
        body: json!({}),
    };

    let err = req_schema.validate(&invalid_parts).unwrap_err();
    let fields: Vec<&str> = err.issues().iter().map(|i| i.field.as_str()).collect();
    assert!(fields.contains(&"headers.authorization"));
    assert!(fields.contains(&"cookies.session_token"));
}
