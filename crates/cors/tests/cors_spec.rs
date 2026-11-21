//! Comprehensive specification test suite for the `cors` crate.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use cors::{
    ACCESS_CONTROL_ALLOW_CREDENTIALS, ACCESS_CONTROL_ALLOW_HEADERS, ACCESS_CONTROL_ALLOW_METHODS,
    ACCESS_CONTROL_ALLOW_ORIGIN, ACCESS_CONTROL_EXPOSE_HEADERS, ACCESS_CONTROL_MAX_AGE,
    ACCESS_CONTROL_REQUEST_HEADERS, ACCESS_CONTROL_REQUEST_METHOD, CorsConfig, CorsConfigBuilder,
    CorsMiddleware, HeaderPolicy, MethodPolicy, ORIGIN, OriginPolicy, VARY, matches_path,
};
use dhap::{Error as DhapError, Pipeline, from_terminal};
use proptest::prelude::*;
use router::{
    HeaderMap, HeaderName, HttpContext, Method, Request, Response, ResponseBody, Router,
    RouterError, StatusCode, from_handler,
};

fn noop_waker() -> Waker {
    fn noop_clone(data: *const ()) -> RawWaker {
        RawWaker::new(data, &NOOP_VTABLE)
    }
    fn noop_action(data: *const ()) {
        let _ = data;
    }
    static NOOP_VTABLE: RawWakerVTable =
        RawWakerVTable::new(noop_clone, noop_action, noop_action, noop_action);

    // SAFETY: RawWaker implementation performs no operations and maintains valid null pointer state.
    unsafe { Waker::from_raw(RawWaker::new(std::ptr::null(), &NOOP_VTABLE)) }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let mut pinned = Pin::from(Box::new(future));
    let waker = noop_waker();
    let mut context = Context::from_waker(&waker);
    match pinned.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => {
            panic!("test future unexpectedly yielded Pending without an external driver")
        }
    }
}

fn make_context(method: Method, path: &str, headers: HeaderMap) -> HttpContext {
    let mut builder = Request::builder().method(method).path(path);
    for (name, values) in headers.iter() {
        for value in values {
            builder = builder.header(name.clone(), value.clone());
        }
    }
    HttpContext::new(builder.build())
}

fn execute_pipeline(
    middleware: CorsMiddleware,
    ctx: &mut HttpContext,
    called: Arc<AtomicBool>,
) -> Result<Response, DhapError<RouterError>> {
    let pipeline = Pipeline::new().pipe(middleware);
    let terminal = from_terminal(move |ctx: &mut HttpContext| {
        called.store(true, Ordering::SeqCst);
        ctx.response.set_status(StatusCode::OK);
        ctx.response.text("downstream output");
        let resp = ctx.response.clone();
        Box::pin(async move { Ok(resp) })
    });
    block_on(pipeline.run(ctx, terminal))
}

#[test]
fn test_non_cors_request_bypasses_middleware() {
    let middleware = CorsMiddleware::default();
    let called = Arc::new(AtomicBool::new(false));
    let mut ctx = make_context(Method::Get, "/api/data", HeaderMap::new());

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    assert!(!response.headers().contains_key(VARY));
}

#[test]
fn test_empty_origin_header_bypasses_middleware() {
    let middleware = CorsMiddleware::default();
    let called = Arc::new(AtomicBool::new(false));
    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "   ");
    let mut ctx = make_context(Method::Get, "/api/data", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(called.load(Ordering::SeqCst));
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[test]
fn test_disabled_middleware_bypasses_all_checks() {
    let config = CorsConfig::builder().enabled(false).build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://example.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "POST");
    let mut ctx = make_context(Method::Options, "/api/data", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::OK);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[test]
fn test_path_scoping_filter() {
    let config = CorsConfig::builder()
        .paths(vec!["/api/*".to_string(), "/webhooks".to_string()])
        .build();
    let middleware = CorsMiddleware::new(config);

    // 1. Matched prefix path: /api/users
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://example.com");
        let mut ctx = make_context(Method::Get, "/api/users", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("*")
        );
    }

    // 2. Unmatched path: /web/home
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://example.com");
        let mut ctx = make_context(Method::Get, "/web/home", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(called.load(Ordering::SeqCst));
        assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    }

    // 3. Matched exact path: /webhooks
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://example.com");
        let mut ctx = make_context(Method::Get, "/webhooks", headers);

        let response = execute_pipeline(middleware, &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("*")
        );
    }
}

#[test]
fn test_matches_path_patterns() {
    assert!(matches_path("*", "/anything/at/all"));
    assert!(matches_path("/*", "/anything/at/all"));
    assert!(matches_path("/api/*", "/api"));
    assert!(matches_path("/api/*", "/api/v1/users"));
    assert!(!matches_path("/api/*", "/apidoc"));
    assert!(matches_path("/auth*", "/auth"));
    assert!(matches_path("/auth*", "/authenticate"));
    assert!(!matches_path("/auth*", "/login"));
    assert!(matches_path("/users", "/users"));
    assert!(!matches_path("/users", "/users/123"));
}

#[test]
fn test_origin_wildcard_without_credentials() {
    let middleware = CorsMiddleware::default();
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://frontend.example.com");
    let mut ctx = make_context(Method::Get, "/api/data", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(called.load(Ordering::SeqCst));
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("*")
    );
    assert!(
        !response
            .headers()
            .contains_key(ACCESS_CONTROL_ALLOW_CREDENTIALS)
    );
    assert!(!response.headers().contains_key(VARY));
}

#[test]
fn test_origin_wildcard_with_credentials_reflects_origin() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .allow_credentials(true)
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://app.example.com");
    let mut ctx = make_context(Method::Get, "/api/profile", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(called.load(Ordering::SeqCst));
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://app.example.com")
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
        Some("true")
    );
    assert_eq!(response.headers().get(VARY), Some("origin"));
}

#[test]
fn test_origin_exact_whitelist_success_and_case_sensitivity() {
    let config = CorsConfig::builder()
        .allow_origins(vec!["https://trusted.com", "https://partner.org"])
        .build();
    let middleware = CorsMiddleware::new(config);

    // 1. Whitelisted origin match
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://trusted.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(called.load(Ordering::SeqCst));
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("https://trusted.com")
        );
        assert_eq!(response.headers().get(VARY), Some("origin"));
    }

    // 2. Case sensitivity: uppercase domain fails per RFC 6454
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://TRUSTED.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(called.load(Ordering::SeqCst));
        assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    }

    // 3. Unauthorized origin
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://unauthorized.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware, &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(called.load(Ordering::SeqCst));
        assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    }
}

#[test]
fn test_origin_subdomain_pattern_matching() {
    let config = CorsConfig::builder()
        .allow_origin_patterns(vec!["https://*.example.com", "https://*.corp.internal"])
        .build();
    let middleware = CorsMiddleware::new(config);

    // 1. Single subdomain
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://api.example.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("https://api.example.com")
        );
    }

    // 2. Nested subdomain
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://sub.portal.corp.internal");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("https://sub.portal.corp.internal")
        );
    }

    // 3. Unmatched domain
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://attacker-example.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware, &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    }
}

#[test]
fn test_origin_dynamic_predicate() {
    let config = CorsConfig::builder()
        .dynamic_origin(|origin, _ctx| origin.ends_with(".trusted.io"))
        .build();
    let middleware = CorsMiddleware::new(config);

    let called = Arc::new(AtomicBool::new(false));
    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://service.trusted.io");
    let mut ctx = make_context(Method::Get, "/api/data", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://service.trusted.io")
    );
}

#[test]
fn test_origin_mirror_policy() {
    let config = CorsConfig::builder().mirror_origin().build();
    let middleware = CorsMiddleware::new(config);

    let called = Arc::new(AtomicBool::new(false));
    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://random-client.net");
    let mut ctx = make_context(Method::Get, "/api/data", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://random-client.net")
    );
    assert_eq!(response.headers().get(VARY), Some("origin"));
}

#[test]
fn test_preflight_valid_request_short_circuits_downstream() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .allow_credentials(true)
        .max_age(120)
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://spa.example.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "POST");
    headers.insert(ACCESS_CONTROL_REQUEST_HEADERS, "authorization, x-trace-id");
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    // Downstream handler must NOT have been called (preflight short-circuit)
    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(response.body().is_empty());

    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://spa.example.com")
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
        Some("true")
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_METHODS),
        Some("GET,HEAD,POST,PUT,PATCH,DELETE,OPTIONS")
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_HEADERS),
        Some("authorization, x-trace-id")
    );
    assert_eq!(response.headers().get(ACCESS_CONTROL_MAX_AGE), Some("120"));

    let vary_values: Vec<&str> = response.headers().get_all(VARY).collect();
    assert!(vary_values.contains(&"origin"));
    assert!(vary_values.contains(&"access-control-request-method"));
    assert!(vary_values.contains(&"access-control-request-headers"));
}

#[test]
fn test_preflight_invalid_origin_fails_closed_without_cors_headers() {
    let config = CorsConfig::builder()
        .allow_origins(vec!["https://trusted.com"])
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://evil.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "POST");
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
    assert!(
        !response
            .headers()
            .contains_key(ACCESS_CONTROL_ALLOW_METHODS)
    );
}

#[test]
fn test_preflight_invalid_method_fails_closed() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .allow_method_list(vec![Method::Get, Method::Post])
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://example.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "DELETE");
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[test]
fn test_preflight_invalid_headers_fails_closed() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .allow_header_list(vec![HeaderName::AUTHORIZATION, HeaderName::CONTENT_TYPE])
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://example.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "POST");
    headers.insert(
        ACCESS_CONTROL_REQUEST_HEADERS,
        "authorization, x-forbidden-token",
    );
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert!(!response.headers().contains_key(ACCESS_CONTROL_ALLOW_ORIGIN));
}

#[test]
fn test_preflight_header_list_case_insensitivity_and_origin_allowance() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .allow_header_list(vec![HeaderName::AUTHORIZATION])
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://example.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "POST");
    // "origin" is allowed implicitly per CORS spec; "AUTHORIZATION" is matched case-insensitively
    headers.insert(ACCESS_CONTROL_REQUEST_HEADERS, "origin, AUTHORIZATION");
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::NO_CONTENT);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("*")
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_HEADERS),
        Some("authorization")
    );
}

#[test]
fn test_preflight_custom_status_code() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .preflight_status(StatusCode::OK)
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://example.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "GET");
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_options_without_request_method_is_not_preflight() {
    let middleware = CorsMiddleware::default();
    let called = Arc::new(AtomicBool::new(false));

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://example.com");
    // Missing Access-Control-Request-Method header -> standard OPTIONS request
    let mut ctx = make_context(Method::Options, "/api/users", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    // Downstream handler must be called for non-preflight OPTIONS requests
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("*")
    );
}

#[test]
fn test_expose_headers_normalization() {
    // 1. Only standard simple response headers: Access-Control-Expose-Headers must be omitted
    {
        let config = CorsConfig::builder()
            .expose_headers(vec![
                HeaderName::CONTENT_TYPE,
                HeaderName::new("cache-control"),
            ])
            .build();
        let middleware = CorsMiddleware::new(config);
        let called = Arc::new(AtomicBool::new(false));

        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://example.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware, &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert!(
            !response
                .headers()
                .contains_key(ACCESS_CONTROL_EXPOSE_HEADERS)
        );
    }

    // 2. Custom headers included: only non-safelisted custom headers are exposed
    {
        let config = CorsConfig::builder()
            .expose_header(HeaderName::new("X-Custom-Total-Count"))
            .expose_header(HeaderName::CONTENT_TYPE)
            .expose_header(HeaderName::new("X-RateLimit-Remaining"))
            .build();
        let middleware = CorsMiddleware::new(config);
        let called = Arc::new(AtomicBool::new(false));

        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://example.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = execute_pipeline(middleware, &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_EXPOSE_HEADERS),
            Some("x-custom-total-count,x-ratelimit-remaining")
        );
    }
}

#[test]
fn test_actual_request_preserves_downstream_payload_and_status() {
    let pipeline = Pipeline::new().pipe(CorsMiddleware::default());
    let terminal = from_terminal(|ctx: &mut HttpContext| {
        ctx.response.set_status(StatusCode::CREATED);
        let _ = ctx.response.json(&serde_json::json!({"id": 42}));
        let resp = ctx.response.clone();
        Box::pin(async move { Ok(resp) })
    });

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://client.com");
    let mut ctx = make_context(Method::Post, "/api/items", headers);

    let response = block_on(pipeline.run(&mut ctx, terminal)).expect("pipeline execution");

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response.body(),
        &ResponseBody::Json("{\"id\":42}".to_string())
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("*")
    );
}

#[test]
fn test_vary_header_idempotency_does_not_duplicate() {
    let config = CorsConfig::builder()
        .allow_origins(vec!["https://trusted.com"])
        .build();
    let pipeline = Pipeline::new().pipe(CorsMiddleware::new(config));

    let terminal = from_terminal(|ctx: &mut HttpContext| {
        let resp = ctx.response.clone();
        Box::pin(async move { Ok(resp) })
    });

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://trusted.com");
    let mut ctx = make_context(Method::Get, "/api/data", headers);
    // Pre-populate response with existing Vary header values including "origin"
    ctx.response.append_header(VARY, "accept-encoding");
    ctx.response.append_header(VARY, "origin");

    let response = block_on(pipeline.run(&mut ctx, terminal)).expect("pipeline execution");

    let vary_entries: Vec<&str> = response.headers().get_all(VARY).collect();
    let origin_count = vary_entries
        .iter()
        .flat_map(|entry| entry.split(','))
        .filter(|part| part.trim().eq_ignore_ascii_case("origin"))
        .count();
    assert_eq!(origin_count, 1);
}

#[test]
fn test_cors_middleware_permissive_preset() {
    let middleware = CorsMiddleware::permissive();
    assert!(middleware.config().credentials);
    assert_eq!(middleware.config().methods, MethodPolicy::Any);

    let called = Arc::new(AtomicBool::new(false));
    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://anywhere.org");
    let mut ctx = make_context(Method::Get, "/api/data", headers);

    let response =
        execute_pipeline(middleware, &mut ctx, Arc::clone(&called)).expect("pipeline execution");

    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://anywhere.org")
    );
    assert_eq!(
        response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
        Some("true")
    );
}

#[test]
fn test_router_integration_with_cors_middleware() {
    let cors = CorsMiddleware::from_builder(
        CorsConfig::builder()
            .allow_origins(vec!["https://dashboard.example.com"])
            .allow_credentials(true),
    );

    let mut router = Router::new();
    router.get(
        "/api/dashboard",
        from_handler(|ctx: &mut HttpContext| {
            Box::pin(async move {
                ctx.response.text("dashboard data");
                Ok(())
            })
        }),
    );

    let compiled = router.compile().expect("compile router");
    let pipeline = Pipeline::new().pipe(cors);

    // 1. Dispatch Preflight OPTIONS request
    {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://dashboard.example.com");
        headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "GET");
        let mut ctx = make_context(Method::Options, "/api/dashboard", headers);

        let compiled_ref = compiled.clone();
        let terminal = from_terminal(move |ctx: &mut HttpContext| {
            let runner = compiled_ref.clone();
            Box::pin(async move {
                runner.dispatch(ctx).await?;
                Ok(ctx.response.clone())
            })
        });

        let response = block_on(pipeline.clone().run(&mut ctx, terminal)).expect("dispatch");

        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("https://dashboard.example.com")
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
            Some("true")
        );
    }

    // 2. Dispatch Actual GET request
    {
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://dashboard.example.com");
        let mut ctx = make_context(Method::Get, "/api/dashboard", headers);

        let compiled_ref = compiled.clone();
        let terminal = from_terminal(move |ctx: &mut HttpContext| {
            let runner = compiled_ref.clone();
            Box::pin(async move {
                runner.dispatch(ctx).await?;
                Ok(ctx.response.clone())
            })
        });

        let response = block_on(pipeline.run(&mut ctx, terminal)).expect("dispatch");

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.body(),
            &ResponseBody::Text("dashboard data".to_string())
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
            Some("https://dashboard.example.com")
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
            Some("true")
        );
    }
}

#[test]
fn test_preflight_no_max_age() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .no_max_age()
        .build();
    let middleware = CorsMiddleware::new(config);
    let called = Arc::new(AtomicBool::new(false));
    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://client.com");
    headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "GET");
    let mut ctx = make_context(Method::Options, "/api/data", headers);

    let response = execute_pipeline(middleware, &mut ctx, called).expect("pipeline execution");
    assert!(!response.headers().contains_key(ACCESS_CONTROL_MAX_AGE));
}

#[test]
fn test_builders_and_debug_representations() {
    let any_origin = OriginPolicy::any();
    assert!(matches!(any_origin, OriginPolicy::Any));

    let mirror_origin = OriginPolicy::mirror();
    assert!(matches!(mirror_origin, OriginPolicy::Mirror));

    let builder = CorsConfigBuilder::default()
        .path("/v1/*")
        .allow_any_methods()
        .mirror_headers()
        .no_max_age();
    let config = builder.build();

    assert!(config.max_age.is_none());
    assert!(config.matches_request_path("/v1/items"));

    let mirror_header = HeaderPolicy::mirror();
    assert_eq!(mirror_header, HeaderPolicy::Mirror);
    let list_header = HeaderPolicy::list([HeaderName::AUTHORIZATION]);
    assert_eq!(
        list_header,
        HeaderPolicy::List(vec![HeaderName::AUTHORIZATION])
    );

    let method_policy_any = MethodPolicy::any();
    assert_eq!(method_policy_any, MethodPolicy::Any);
    let method_policy_list = MethodPolicy::list([Method::Get]);
    assert_eq!(method_policy_list, MethodPolicy::List(vec![Method::Get]));

    let debug_origin_any = format!("{:?}", OriginPolicy::Any);
    assert_eq!(debug_origin_any, "OriginPolicy::Any");

    let debug_origin_exact = format!("{:?}", OriginPolicy::exact(["https://a.com"]));
    assert!(debug_origin_exact.contains("https://a.com"));

    let debug_origin_pattern = format!("{:?}", OriginPolicy::pattern(["*.a.com"]));
    assert!(debug_origin_pattern.contains("*.a.com"));

    let debug_origin_mirror = format!("{:?}", OriginPolicy::Mirror);
    assert_eq!(debug_origin_mirror, "OriginPolicy::Mirror");

    let debug_origin_dynamic = format!("{:?}", OriginPolicy::dynamic(|_, _| true));
    assert!(debug_origin_dynamic.contains("<closure>"));

    let debug_middleware = format!("{:?}", CorsMiddleware::default());
    assert!(debug_middleware.contains("CorsMiddleware"));
}

#[test]
fn test_preflight_and_actual_request_with_custom_exposed_headers() {
    let config = CorsConfig::builder()
        .allow_any_origin()
        .allow_credentials(true)
        .expose_header(HeaderName::new("X-Server-Time"))
        .build();
    let middleware = CorsMiddleware::new(config);

    // 1. Preflight response sets Access-Control-Expose-Headers
    {
        let called = Arc::new(AtomicBool::new(false));
        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://client.com");
        headers.insert(ACCESS_CONTROL_REQUEST_METHOD, "GET");
        let mut ctx = make_context(Method::Options, "/api/data", headers);

        let response = execute_pipeline(middleware.clone(), &mut ctx, Arc::clone(&called))
            .expect("pipeline execution");

        assert_eq!(
            response.headers().get(ACCESS_CONTROL_EXPOSE_HEADERS),
            Some("x-server-time")
        );
    }

    // 2. Actual request response preserves Access-Control-Expose-Headers
    {
        let pipeline = Pipeline::new().pipe(middleware);
        let terminal = from_terminal(|ctx: &mut HttpContext| {
            ctx.response.set_status(StatusCode::OK);
            ctx.response.text("payload");
            let resp = ctx.response.clone();
            Box::pin(async move { Ok(resp) })
        });

        let mut headers = HeaderMap::new();
        headers.insert(ORIGIN, "https://client.com");
        let mut ctx = make_context(Method::Get, "/api/data", headers);

        let response = block_on(pipeline.run(&mut ctx, terminal)).expect("pipeline execution");

        assert_eq!(
            response.headers().get(ACCESS_CONTROL_EXPOSE_HEADERS),
            Some("x-server-time")
        );
        assert_eq!(
            response.headers().get(ACCESS_CONTROL_ALLOW_CREDENTIALS),
            Some("true")
        );
    }
}

#[test]
fn test_downstream_error_preserves_cors_headers_on_context() {
    let middleware = CorsMiddleware::permissive();
    let pipeline = Pipeline::new().pipe(middleware);
    let terminal = from_terminal(|ctx: &mut HttpContext| {
        let _ = ctx;
        Box::pin(async move { Err(RouterError::NotFound) })
    });

    let mut headers = HeaderMap::new();
    headers.insert(ORIGIN, "https://client.com");
    let mut ctx = make_context(Method::Get, "/api/data", headers);

    let result = block_on(pipeline.run(&mut ctx, terminal));
    assert!(result.is_err());
    assert_eq!(
        ctx.response.headers().get(ACCESS_CONTROL_ALLOW_ORIGIN),
        Some("https://client.com")
    );
}

proptest! {
    #[test]
    fn proptest_matches_path_wildcard(suffix in "[a-z0-9/_]{1,30}") {
        let pattern = "/api/*";
        let path = format!("/api/{suffix}");
        prop_assert!(matches_path(pattern, &path));
    }

    #[test]
    fn proptest_header_case_insensitivity(header_name in "[a-z-]{1,20}") {
        let upper = header_name.to_ascii_uppercase();
        let config = CorsConfig::builder()
            .allow_header_list(vec![HeaderName::new(&header_name)])
            .build();
        prop_assert!(config.are_headers_allowed(&[&upper]));
    }
}
