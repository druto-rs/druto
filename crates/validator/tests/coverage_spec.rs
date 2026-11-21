//! Targeted test suite to exercise all internal modules, edge cases, and async methods for maximum test coverage.

use serde::{Deserialize, Serialize};
use serde_json::json;
use validator::{coercion::*, prelude::*};

#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
struct MockDto {
    name: String,
    score: i64,
}

#[test]
fn test_coercion_edge_cases() {
    assert_eq!(trim_whitespace("  hello  "), "hello");

    let empty_str = json!("   ");
    assert_eq!(empty_string_to_null(&empty_str), serde_json::Value::Null);
    let non_empty_str = json!("val");
    assert_eq!(empty_string_to_null(&non_empty_str), non_empty_str);

    let nested_tree = json!({
        "a": "  ",
        "b": ["", "valid", "   "],
        "c": { "inner": "  ", "num": 10 }
    });
    let normalized = normalize_empty_strings(&nested_tree);
    assert_eq!(normalized["a"], serde_json::Value::Null);
    assert_eq!(normalized["b"][0], serde_json::Value::Null);
    assert_eq!(normalized["b"][1], "valid");
    assert_eq!(normalized["b"][2], serde_json::Value::Null);
    assert_eq!(normalized["c"]["inner"], serde_json::Value::Null);
    assert_eq!(normalized["c"]["num"], 10);

    // Number coercions
    assert_eq!(coerce_to_number(&json!(42)).unwrap().as_i64(), Some(42));
    assert_eq!(coerce_to_number(&json!("100")).unwrap().as_i64(), Some(100));
    assert_eq!(
        coerce_to_number(&json!("18446744073709551615"))
            .unwrap()
            .as_u64(),
        Some(18446744073709551615)
    );
    assert!(
        (coerce_to_number(&json!("12.345"))
            .unwrap()
            .as_f64()
            .unwrap()
            - 12.345)
            .abs()
            < 1e-4
    );
    assert!(coerce_to_number(&json!("not-a-number")).is_none());
    assert!(coerce_to_number(&json!(true)).is_none());

    // Bool coercions
    assert_eq!(coerce_to_bool(&json!(true)), Some(true));
    assert_eq!(coerce_to_bool(&json!(false)), Some(false));
    assert_eq!(coerce_to_bool(&json!(1)), Some(true));
    assert_eq!(coerce_to_bool(&json!(0)), Some(false));
    assert_eq!(coerce_to_bool(&json!(2)), None);
    assert_eq!(coerce_to_bool(&json!(1.5)), None);
    assert_eq!(coerce_to_bool(&json!("1")), Some(true));
    assert_eq!(coerce_to_bool(&json!("yes")), Some(true));
    assert_eq!(coerce_to_bool(&json!("0")), Some(false));
    assert_eq!(coerce_to_bool(&json!("no")), Some(false));
    assert_eq!(coerce_to_bool(&json!("invalid")), None);
    assert_eq!(coerce_to_bool(&json!([])), None);

    // String coercions
    assert_eq!(coerce_to_string(&json!("test")), Some("test".to_string()));
    assert_eq!(coerce_to_string(&json!(123)), Some("123".to_string()));
    assert_eq!(coerce_to_string(&json!(true)), Some("true".to_string()));
    assert_eq!(coerce_to_string(&json!([])), None);

    // Array coercions
    assert_eq!(
        coerce_to_array(&json!([1, 2, 3])),
        Some(vec![json!(1), json!(2), json!(3)])
    );
    assert_eq!(coerce_to_array(&json!(null)), None);
    assert_eq!(
        coerce_to_array(&json!("single")),
        Some(vec![json!("single")])
    );
}

#[test]
fn test_validation_context_full_api() {
    #[derive(Debug, PartialEq, Eq, Clone)]
    struct Dummy(i32);

    let mut ctx = ValidationContext::new();
    assert!(ctx.is_empty());
    assert_eq!(ctx.len(), 0);

    ctx.insert(Dummy(42));
    assert!(!ctx.is_empty());
    assert_eq!(ctx.len(), 1);
    assert!(ctx.contains::<Dummy>());
    assert_eq!(ctx.get::<Dummy>(), Some(&Dummy(42)));

    let removed = ctx.remove::<Dummy>();
    assert!(removed);
    assert!(!ctx.contains::<Dummy>());

    ctx.insert(Dummy(999));
    ctx.clear();
    assert!(ctx.is_empty());

    let formatted = format!("{ctx:?}");
    assert!(formatted.contains("ValidationContext"));
}

#[test]
fn test_error_and_issue_full_api() {
    let mut issue = ValidationIssue::new("field", "rule", "message");
    assert_eq!(issue.field, "field");
    assert_eq!(issue.rule, "rule");
    assert_eq!(issue.message, "message");

    let mut params = RuleParams::new();
    params.insert("k", "v");
    issue = issue.with_params(params.clone());
    assert_eq!(issue.params.get("k"), Some("v"));

    let mut err = ValidationError::single("f", "r", "m");
    assert_eq!(err.len(), 1);
    assert!(!err.is_empty());
    assert_eq!(err.first_issue().unwrap().field, "f");

    err.push(ValidationIssue::new("f2", "r2", "m2"));
    assert_eq!(err.len(), 2);
    assert_eq!(err.issues().len(), 2);
    assert_eq!(err.issues_mut().len(), 2);

    let mut other_err = ValidationError::new();
    other_err.push(ValidationIssue::new("f3", "r3", "m3"));
    err.merge(other_err);
    assert_eq!(err.len(), 3);

    // Iterators
    let mut count = 0;
    for _ in &err {
        count += 1;
    }
    assert_eq!(count, 3);

    let issues_vec = err.into_issues();
    assert_eq!(issues_vec.len(), 3);

    let reconstructed = ValidationError::from_issues(issues_vec);
    assert_eq!(reconstructed.len(), 3);

    // Display & Error
    let display_str = format!("{reconstructed}");
    assert!(display_str.contains("Validation failed with 3 errors"));
    let empty_err = ValidationError::new();
    assert_eq!(format!("{empty_err}"), "Validation failed with 0 issues");

    // JSON serialization
    let json_val = reconstructed.to_json_value();
    assert!(json_val.get("errors").is_some());
    let json_str = reconstructed.to_json_string();
    assert!(json_str.contains("errors"));

    // Problem details without instance
    let problem_no_instance = reconstructed.to_problem_details(None);
    assert!(problem_no_instance.get("instance").is_none());
}

#[test]
fn test_rule_params_api() {
    let mut params = RuleParams::new();
    assert!(params.is_empty());
    params.insert("param1", "val1");
    assert_eq!(params.len(), 1);
    assert!(params.contains("param1"));
    assert_eq!(params.get("param1"), Some("val1"));

    let removed = params.remove("param1");
    assert_eq!(removed, Some("val1".to_string()));
    assert!(!params.contains("param1"));
}

#[test]
fn test_request_parts_fluent_builders_and_decodes() {
    let parts = RequestParts::new()
        .with_params(json!({ "id": 1 }))
        .with_query(json!({ "q": "search" }))
        .with_headers(json!({ "x-key": "secret" }))
        .with_cookies(json!({ "auth": "token" }))
        .with_body(json!({ "name": "Druto", "score": 95 }));

    assert_eq!(parts.params["id"], 1);
    assert_eq!(parts.query["q"], "search");
    assert_eq!(parts.headers["x-key"], "secret");
    assert_eq!(parts.cookies["auth"], "token");
    assert_eq!(parts.body["score"], 95);

    let req_schema = request()
        .params(object().field("id", integer()))
        .query(object().field("q", string()))
        .headers(object().field("x-key", string()))
        .cookies(object().field("auth", string()))
        .body(object().field("name", string()).field("score", integer()))
        .bail(true);

    let validated = req_schema.validate(&parts).unwrap();
    let body: MockDto = validated.decode_body().unwrap();
    assert_eq!(body.name, "Druto");
    assert_eq!(body.score, 95);

    let decoded_headers: serde_json::Value = validated.decode_headers().unwrap();
    assert_eq!(decoded_headers["x-key"], "secret");

    let decoded_cookies: serde_json::Value = validated.decode_cookies().unwrap();
    assert_eq!(decoded_cookies["auth"], "token");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_async_all_schema_types() {
    // Array async
    let arr_schema = array(string().min_len(2))
        .min_items(1)
        .max_items(5)
        .exact_items(2)
        .distinct()
        .not_empty()
        .bail(true)
        .with_message("Array error");

    let res_arr = arr_schema.validate_async(&json!(["ab", "cd"])).await;
    assert_eq!(res_arr.unwrap(), vec!["ab".to_string(), "cd".to_string()]);

    let err_arr = arr_schema.validate_async(&json!(["a"])).await.unwrap_err();
    assert!(!err_arr.is_empty());

    // Record async
    let rec_schema = record(integer().positive())
        .bail(true)
        .with_message("Record error");

    let valid_rec = json!({ "k1": 10, "k2": 20 });
    assert!(rec_schema.validate_async(&valid_rec).await.is_ok());

    let invalid_rec = json!({ "k1": -5 });
    assert!(rec_schema.validate_async(&invalid_rec).await.is_err());

    // Tuple2 async
    let tup_schema = tuple2(string(), integer()).bail(true);
    let valid_tup = json!(["hello", 42]);
    let (s, n) = tup_schema.validate_async(&valid_tup).await.unwrap();
    assert_eq!(s, "hello");
    assert_eq!(n, 42);

    let invalid_tup = json!(["hello"]);
    assert!(tup_schema.validate_async(&invalid_tup).await.is_err());

    // Union2 async
    let uni_schema = union2(integer().map(|i| i.to_string()), string());
    assert_eq!(uni_schema.validate_async(&json!(123)).await.unwrap(), "123");
    assert_eq!(
        uni_schema.validate_async(&json!("abc")).await.unwrap(),
        "abc"
    );

    // Optional, Nullable, Default async
    let opt_s = string().optional();
    assert_eq!(opt_s.validate_async(&json!(null)).await.unwrap(), None);

    let nul_s = integer().nullable();
    assert_eq!(nul_s.validate_async(&json!(null)).await.unwrap(), None);

    let def_s = string().default_value("fallback".to_string());
    assert_eq!(
        def_s.validate_async(&json!(null)).await.unwrap(),
        "fallback"
    );

    // Transform and Refine async
    let trans_s = string().transform(|s| Ok(s.len()));
    assert_eq!(trans_s.validate_async(&json!("druto")).await.unwrap(), 5);

    let ref_s = integer().refine(|n| *n > 0);
    assert_eq!(ref_s.validate_async(&json!(10)).await.unwrap(), 10);
    assert!(ref_s.validate_async(&json!(-10)).await.is_err());

    // Object async & decode_async
    let obj_s = object()
        .field("name", string())
        .field("score", integer())
        .bail(true)
        .with_message("Object failure");

    let valid_obj = json!({ "name": "Hero", "score": 100 });
    let dto: MockDto = obj_s.decode_async(&valid_obj).await.unwrap();
    assert_eq!(dto.name, "Hero");
    assert_eq!(dto.score, 100);

    let refined_obj =
        obj_s.refine(|val| val.get("score").and_then(|s| s.as_i64()).unwrap_or(0) >= 50);
    let failing_refine = json!({ "name": "Low", "score": 10 });
    assert!(refined_obj.validate_async(&failing_refine).await.is_err());
}

#[test]
fn test_rule_context_helpers_and_siblings() {
    let mut ambient_ctx = ValidationContext::new();
    ambient_ctx.insert("tenant_123".to_string());

    let parent_json = json!({
        "password": "secret",
        "confirm": "secret",
        "role": "admin"
    });

    let messages = SimpleMessagesProvider::default();
    let rule_ctx = RuleContext::new("confirm", &ambient_ctx, Some(&parent_json), &messages);

    assert_eq!(rule_ctx.path(), "confirm");
    assert_eq!(
        rule_ctx.get_ambient::<String>(),
        Some(&"tenant_123".to_string())
    );
    assert_eq!(rule_ctx.context().len(), 1);
    assert!(rule_ctx.parent_data().is_some());
    assert_eq!(
        rule_ctx.get_sibling("role").and_then(|v| v.as_str()),
        Some("admin")
    );
    assert_eq!(rule_ctx.get_sibling("non_existent"), None);

    let issue = rule_ctx.create_issue("custom_rule", RuleParams::new(), Some("Manual override"));
    assert_eq!(issue.message, "Manual override");
    assert_eq!(issue.field, "confirm");
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_enum_schema_all_branches() {
    #[derive(Debug, PartialEq, Eq)]
    enum Status {
        Active,
        Inactive,
    }

    impl std::str::FromStr for Status {
        type Err = &'static str;
        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "active" => Ok(Self::Active),
                "inactive" => Ok(Self::Inactive),
                _ => Err("invalid status"),
            }
        }
    }

    let default_enum = r#enum::<Status>().bail(true).with_message("Status error");

    // Async validation
    assert_eq!(
        default_enum.validate_async(&json!("active")).await.unwrap(),
        Status::Active
    );

    // Parse error without preconfigured variants
    let err_unregistered = default_enum.validate(&json!("banned")).unwrap_err();
    assert_eq!(err_unregistered.issues()[0].message, "Status error");

    // Null and non-string inputs
    assert!(default_enum.validate(&json!(null)).is_err());
    assert!(default_enum.validate(&json!(12345)).is_err());
}

#[cfg(feature = "async")]
#[tokio::test]
async fn test_array_and_number_async_and_predicates() {
    let s = array(integer().positive())
        .min_items(2)
        .max_items(4)
        .exact_items(3)
        .bail(true);

    // Async with failing exact items
    let err_exact = s.validate_async(&json!([1, 2])).await.unwrap_err();
    assert_eq!(err_exact.issues()[0].rule, "exact_items");

    // Async with failing element
    let err_elem = s.validate_async(&json!([1, -2, 3])).await.unwrap_err();
    assert_eq!(err_elem.issues()[0].field, "[1]");

    // Number float predicates and async
    let num_s = number()
        .min(5.5)
        .max(10.5)
        .range(6.0, 10.0)
        .multiple_of(0.5)
        .bail(true)
        .with_message("Float number error");

    assert_eq!(num_s.validate_async(&json!(7.5)).await.unwrap(), 7.5);
    let err_mult = num_s.validate_async(&json!(7.3)).await.unwrap_err();
    assert_eq!(err_mult.issues()[0].message, "Float number error");

    // Integer negative
    let int_neg = integer()
        .negative()
        .bail(true)
        .with_message("Must be negative");
    assert_eq!(int_neg.validate(&json!(-10)).unwrap(), -10);
    let err_pos = int_neg.validate(&json!(10)).unwrap_err();
    assert_eq!(err_pos.issues()[0].message, "Must be negative");
}

#[cfg(feature = "router")]
#[tokio::test]
async fn test_router_async_and_headers_validation() {
    use router::{context::HttpContext, request::Request};

    let req = Request::builder()
        .path("/test")
        .header("x-app-id", "druto-core")
        .json(&json!({ "name": "AsyncUser", "score": 88 }))
        .unwrap()
        .build();

    let mut ctx = HttpContext::new(req);

    // Validate headers
    let header_schema = object().field("x-app-id", string().starts_with("druto-"));
    let headers_val = ctx.validate_headers(&header_schema).unwrap();
    assert_eq!(headers_val["x-app-id"], "druto-core");

    // Validate body async
    let body_schema = object().field("name", string()).field("score", integer());
    let validated_body: MockDto = ctx
        .validate_body_async(&body_schema.clone().decode_into::<MockDto>())
        .await
        .unwrap();
    assert_eq!(validated_body.name, "AsyncUser");
    assert_eq!(validated_body.score, 88);

    // Validate request async
    let req_schema = request().headers(header_schema).body(body_schema);

    let val_req = ctx.validate_request_async(&req_schema).await.unwrap();
    let decoded_dto: MockDto = val_req.decode_body().unwrap();
    assert_eq!(decoded_dto.name, "AsyncUser");

    // RouterError from ValidationError
    let err = ValidationError::single("f", "r", "msg");
    let router_err: router::RouterError = err.into();
    assert!(format!("{router_err}").contains("msg"));
}
