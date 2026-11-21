//! Comprehensive tests for ObjectSchema, unknown field policies, cross-field combinators, and decoding.

use serde::Deserialize;
use serde_json::json;
use validator::prelude::*;

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct UserProfile {
    username: String,
    age: i64,
    email: Option<String>,
}

#[derive(Debug, PartialEq, Eq, Deserialize)]
struct PasswordChangeForm {
    password: String,
    password_confirmation: String,
}

#[test]
fn test_object_strip_policy_default() {
    let s = object()
        .field("username", string().min_len(3))
        .field("age", integer().positive());

    let input = json!({
        "username": "hitarashi",
        "age": 28,
        "is_admin": true, // extra field
        "injected": "malicious"
    });

    let validated = s.validate(&input).unwrap();
    // Default is strip: extra fields must be removed
    assert_eq!(
        validated.get("username").and_then(|v| v.as_str()),
        Some("hitarashi")
    );
    assert_eq!(validated.get("age").and_then(|v| v.as_i64()), Some(28));
    assert!(validated.get("is_admin").is_none());
    assert!(validated.get("injected").is_none());
}

#[test]
fn test_object_strict_policy() {
    let s = object().field("title", string()).strict();

    let valid_input = json!({
        "title": "Clean Architecture"
    });
    assert!(s.validate(&valid_input).is_ok());

    let dirty_input = json!({
        "title": "Clean Architecture",
        "unknown_extra": "rejected"
    });
    let err = s.validate(&dirty_input).unwrap_err();
    assert_eq!(err.len(), 1);
    assert_eq!(err.issues()[0].rule, "unknown_field");
    assert_eq!(err.issues()[0].field, "unknown_extra");
}

#[test]
fn test_object_passthrough_policy() {
    let s = object().field("title", string()).passthrough();

    let input = json!({
        "title": "Framework",
        "metadata": { "env": "production" }
    });
    let validated = s.validate(&input).unwrap();
    assert_eq!(
        validated.get("title").and_then(|v| v.as_str()),
        Some("Framework")
    );
    assert!(validated.get("metadata").is_some());
}

#[test]
fn test_object_decode_typed() {
    let s = object()
        .field("username", string().min_len(2))
        .field("age", integer().positive())
        .field("email", string().email().optional());

    let input = json!({
        "username": "druto_user",
        "age": 30,
        "email": "user@example.com",
        "unknown_junk": 999
    });

    let profile: UserProfile = s.decode(&input).unwrap();
    assert_eq!(profile.username, "druto_user");
    assert_eq!(profile.age, 30);
    assert_eq!(profile.email, Some("user@example.com".to_string()));
}

#[test]
fn test_object_cross_field_same_as() {
    let s = object()
        .field("password", string().min_len(6))
        .field("password_confirmation", string().min_len(6))
        .same_as("password_confirmation", "password");

    let matching = json!({
        "password": "secret_password",
        "password_confirmation": "secret_password"
    });
    let form: PasswordChangeForm = s.decode(&matching).unwrap();
    assert_eq!(form.password, "secret_password");

    let mismatched = json!({
        "password": "secret_password",
        "password_confirmation": "different_password"
    });
    let err = s.validate(&mismatched).unwrap_err();
    assert_eq!(err.issues()[0].rule, "same_as");
    assert_eq!(err.issues()[0].field, "password_confirmation");
}

#[test]
fn test_object_cross_field_different_from() {
    let s = object()
        .field("current_password", string())
        .field("new_password", string())
        .different_from("new_password", "current_password");

    let same_pass = json!({
        "current_password": "same_old_pass",
        "new_password": "same_old_pass"
    });
    let err = s.validate(&same_pass).unwrap_err();
    assert_eq!(err.issues()[0].rule, "different_from");
    assert_eq!(err.issues()[0].field, "new_password");

    let different_pass = json!({
        "current_password": "same_old_pass",
        "new_password": "brand_new_pass"
    });
    assert!(s.validate(&different_pass).is_ok());
}

#[test]
fn test_object_cross_field_required_if() {
    let s = object()
        .field("payment_type", string())
        .field("card_number", string().optional())
        .required_if("card_number", "payment_type", json!("credit_card"));

    let card_missing = json!({
        "payment_type": "credit_card"
    });
    let err = s.validate(&card_missing).unwrap_err();
    assert_eq!(err.issues()[0].rule, "required_if");
    assert_eq!(err.issues()[0].field, "card_number");

    let card_provided = json!({
        "payment_type": "credit_card",
        "card_number": "4111111111111111"
    });
    assert!(s.validate(&card_provided).is_ok());

    let cash_type = json!({
        "payment_type": "cash"
    });
    assert!(s.validate(&cash_type).is_ok());
}

#[test]
fn test_object_cross_field_required_without() {
    let s = object()
        .field("email", string().optional())
        .field("phone", string().optional())
        .required_without("phone", "email");

    let neither_provided = json!({});
    let err = s.validate(&neither_provided).unwrap_err();
    assert_eq!(err.issues()[0].rule, "required_without");
    assert_eq!(err.issues()[0].field, "phone");

    let email_provided = json!({
        "email": "user@example.com"
    });
    assert!(s.validate(&email_provided).is_ok());

    let phone_provided = json!({
        "phone": "+1234567890"
    });
    assert!(s.validate(&phone_provided).is_ok());
}

#[test]
fn test_object_cross_field_after_and_before_date() {
    let s = object()
        .field("start_date", string())
        .field("end_date", string())
        .after_field("end_date", "start_date")
        .before_field("start_date", "end_date");

    let valid_range = json!({
        "start_date": "2026-01-01",
        "end_date": "2026-06-01"
    });
    assert!(s.validate(&valid_range).is_ok());

    let invalid_range = json!({
        "start_date": "2026-10-01",
        "end_date": "2026-05-01"
    });
    let err = s.validate(&invalid_range).unwrap_err();
    assert!(
        err.issues()
            .iter()
            .any(|i| i.rule == "after" || i.rule == "before")
    );
}

#[test]
fn test_nested_object_paths() {
    let s = object().field(
        "user",
        object().field("profile", object().field("name", string().min_len(5))),
    );

    let invalid_nested = json!({
        "user": {
            "profile": {
                "name": "bob"
            }
        }
    });

    let err = s.validate(&invalid_nested).unwrap_err();
    assert_eq!(err.issues()[0].field, "user.profile.name");
    assert_eq!(err.issues()[0].rule, "min_len");
}

#[test]
fn test_custom_messages_provider_and_placeholders() {
    let s = object().field("username", string().min_len(5)).messages(&[
        ("required", "Field ':field' is mandatory."),
        (
            "min_len",
            "Field ':field' must contain at least :min chars.",
        ),
    ]);

    let short_name = json!({
        "username": "joe"
    });
    let err = s.validate(&short_name).unwrap_err();
    assert_eq!(
        err.issues()[0].message,
        "Field 'username' must contain at least 5 chars."
    );
}

#[test]
fn test_simple_messages_provider_direct() {
    let mut provider = SimpleMessagesProvider::new();
    provider.set_rule_default("required", "Custom required for :field");
    provider.set_field_rule("email", "email", "Email field :field is malformed!");

    let mut params = RuleParams::new();
    params.insert("field", "email");
    let rendered = provider.format_message("email", "email", &params, None);
    assert_eq!(rendered, "Email field email is malformed!");
}

#[test]
fn test_validation_context_typemap() {
    #[derive(Clone, Debug, PartialEq)]
    struct TenantId(String);

    let mut context = ValidationContext::new();
    context.insert(TenantId("tenant_acme".to_string()));

    let s = object().field(
        "resource",
        string().refine_with(
            |name| !name.is_empty(),
            "valid_resource",
            "Resource name is invalid",
        ),
    );

    assert_eq!(context.get::<TenantId>().unwrap().0, "tenant_acme");
    let result = s.validate_with_context(&json!({ "resource": "assets" }), &context);
    assert!(result.is_ok());
}
