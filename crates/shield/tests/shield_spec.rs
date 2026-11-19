//! Comprehensive specification test suite for the `shield` crate.

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    task::{Context, Poll, RawWaker, RawWakerVTable, Waker},
};

use base64::prelude::*;
use cookie::SameSite;
use crypto::key::Key;
use pipeline::{Pipeline, PipelineError, from_terminal};
use router::{HeaderName, HttpContext, Method, Request, Response, RouterError, StatusCode};
use session::{Session, SessionContext, SessionId};
use shield::{
    CrossDomainPolicy, CspConfig, CspNonce, CsrfConfig, CsrfFailureMode, CsrfToken, FrameAction,
    HstsConfig, PermissionsPolicyBuilder, ReferrerPolicy, SecurityHeadersConfig, SecurityPreset,
    ShieldConfig, ShieldError, ShieldMiddleware,
    constants::*,
    extract_csrf_token, generate_csrf_token,
    middleware::{CspMiddleware, CsrfMiddleware, SecurityHeadersMiddleware},
    verify_csrf_token,
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

fn make_context(method: Method, path: &str) -> HttpContext {
    let request = Request::builder().method(method).path(path).build();
    HttpContext::new(request)
}

fn make_context_with_headers(method: Method, path: &str, headers: &[(&str, &str)]) -> HttpContext {
    let mut builder = Request::builder().method(method).path(path);
    for (k, v) in headers {
        builder = builder.header(*k, *v);
    }
    HttpContext::new(builder.build())
}

fn make_context_with_body(
    method: Method,
    path: &str,
    body: impl Into<Vec<u8>>,
    content_type: &str,
) -> HttpContext {
    let request = Request::builder()
        .method(method)
        .path(path)
        .header("content-type", content_type)
        .body(body.into())
        .build();
    HttpContext::new(request)
}

// -----------------------------------------------------------------------------
// 1. Constants Verification
// -----------------------------------------------------------------------------

#[test]
fn test_canonical_header_constants() {
    assert_eq!(CONTENT_SECURITY_POLICY, "content-security-policy");
    assert_eq!(
        CONTENT_SECURITY_POLICY_REPORT_ONLY,
        "content-security-policy-report-only"
    );
    assert_eq!(STRICT_TRANSPORT_SECURITY, "strict-transport-security");
    assert_eq!(X_CONTENT_TYPE_OPTIONS, "x-content-type-options");
    assert_eq!(X_FRAME_OPTIONS, "x-frame-options");
    assert_eq!(REFERRER_POLICY, "referrer-policy");
    assert_eq!(PERMISSIONS_POLICY, "permissions-policy");
    assert_eq!(X_XSS_PROTECTION, "x-xss-protection");
    assert_eq!(X_DOWNLOAD_OPTIONS, "x-download-options");
    assert_eq!(
        X_PERMITTED_CROSS_DOMAIN_POLICIES,
        "x-permitted-cross-domain-policies"
    );
    assert_eq!(CROSS_ORIGIN_EMBEDDER_POLICY, "cross-origin-embedder-policy");
    assert_eq!(CROSS_ORIGIN_OPENER_POLICY, "cross-origin-opener-policy");
    assert_eq!(CROSS_ORIGIN_RESOURCE_POLICY, "cross-origin-resource-policy");
    assert_eq!(ORIGIN_AGENT_CLUSTER, "origin-agent-cluster");
    assert_eq!(CLEAR_SITE_DATA, "clear-site-data");
    assert_eq!(CACHE_CONTROL, "cache-control");
    assert_eq!(X_CSRF_TOKEN, "x-csrf-token");
    assert_eq!(X_XSRF_TOKEN, "x-xsrf-token");
    assert_eq!(DEFAULT_XSRF_COOKIE_NAME, "XSRF-TOKEN");
    assert_eq!(DEFAULT_CSRF_SESSION_KEY, "druto.csrf.secret");
    assert_eq!(DEFAULT_CSRF_FIELD_NAME, "_csrf");
    assert_eq!(DEFAULT_CSRF_TOKEN_FIELD_NAME, "_token");
}

// -----------------------------------------------------------------------------
// 2. Error Taxonomy & Conversions
// -----------------------------------------------------------------------------

#[test]
fn test_shield_error_display_and_traits() {
    let missing_csrf = ShieldError::MissingCsrfToken;
    let invalid_csrf = ShieldError::InvalidCsrfToken;
    let missing_secret = ShieldError::MissingCsrfSecret;
    let invalid_config = ShieldError::InvalidConfiguration("bad option".to_string());

    assert_eq!(
        missing_csrf.to_string(),
        "403 Forbidden: Missing required CSRF token"
    );
    assert_eq!(
        invalid_csrf.to_string(),
        "403 Forbidden: Invalid or expired CSRF token"
    );
    assert_eq!(
        missing_secret.to_string(),
        "500 Internal Server Error: Missing CSRF secret in session"
    );
    assert_eq!(
        invalid_config.to_string(),
        "Invalid shield configuration: bad option"
    );

    // From<ShieldError> for RouterError
    let router_err1: RouterError = missing_csrf.clone().into();
    let router_err2: RouterError = invalid_csrf.clone().into();
    let router_err3: RouterError = missing_secret.clone().into();
    let router_err4: RouterError = invalid_config.clone().into();

    assert_eq!(router_err1, RouterError::InvalidSignature);
    assert_eq!(router_err2, RouterError::InvalidSignature);
    assert_eq!(
        router_err3,
        RouterError::Internal(missing_secret.to_string())
    );
    assert_eq!(
        router_err4,
        RouterError::Internal(invalid_config.to_string())
    );

    assert_eq!(missing_csrf, ShieldError::MissingCsrfToken);
    assert_ne!(missing_csrf, invalid_csrf);
    assert_eq!(missing_csrf.clone(), ShieldError::MissingCsrfToken);
    assert!(format!("{missing_csrf:?}").contains("MissingCsrfToken"));
}

// -----------------------------------------------------------------------------
// 3. Security Headers Policies & Presets
// -----------------------------------------------------------------------------

#[test]
fn test_frame_action_as_str() {
    assert_eq!(FrameAction::Deny.as_str(), "DENY");
    assert_eq!(FrameAction::SameOrigin.as_str(), "SAMEORIGIN");
}

#[test]
fn test_referrer_policy_as_str() {
    assert_eq!(ReferrerPolicy::NoReferrer.as_str(), "no-referrer");
    assert_eq!(
        ReferrerPolicy::NoReferrerWhenDowngrade.as_str(),
        "no-referrer-when-downgrade"
    );
    assert_eq!(ReferrerPolicy::Origin.as_str(), "origin");
    assert_eq!(
        ReferrerPolicy::OriginWhenCrossOrigin.as_str(),
        "origin-when-cross-origin"
    );
    assert_eq!(ReferrerPolicy::SameOrigin.as_str(), "same-origin");
    assert_eq!(ReferrerPolicy::StrictOrigin.as_str(), "strict-origin");
    assert_eq!(
        ReferrerPolicy::StrictOriginWhenCrossOrigin.as_str(),
        "strict-origin-when-cross-origin"
    );
    assert_eq!(ReferrerPolicy::UnsafeUrl.as_str(), "unsafe-url");
}

#[test]
fn test_cross_domain_policy_as_str() {
    assert_eq!(CrossDomainPolicy::None.as_str(), "none");
    assert_eq!(CrossDomainPolicy::MasterOnly.as_str(), "master-only");
    assert_eq!(CrossDomainPolicy::ByContentType.as_str(), "by-content-type");
    assert_eq!(CrossDomainPolicy::All.as_str(), "all");
}

#[test]
fn test_hsts_config_formatting() {
    let default_hsts = HstsConfig::default();
    assert_eq!(
        default_hsts.to_header_value(),
        "max-age=31536000; includeSubDomains"
    );

    let custom_hsts = HstsConfig {
        max_age: 15_552_000,
        include_subdomains: false,
        preload: true,
    };
    assert_eq!(custom_hsts.to_header_value(), "max-age=15552000; preload");

    let full_hsts = HstsConfig {
        max_age: 63_072_000,
        include_subdomains: true,
        preload: true,
    };
    assert_eq!(
        full_hsts.to_header_value(),
        "max-age=63072000; includeSubDomains; preload"
    );
}

#[test]
fn test_permissions_policy_builder() {
    let policy = PermissionsPolicyBuilder::new()
        .deny("camera")
        .deny("microphone")
        .allow_self("geolocation")
        .allow_all("fullscreen")
        .build();

    assert!(policy.contains("camera=()"));
    assert!(policy.contains("microphone=()"));
    assert!(policy.contains("geolocation=(self)"));
    assert!(policy.contains("fullscreen=*"));
}

#[test]
fn test_security_preset_compilation() {
    // 1. SecureDefaults
    let secure_defaults = SecurityHeadersConfig::from_preset(SecurityPreset::SecureDefaults);
    let compiled = secure_defaults.compile_headers();
    let map: std::collections::HashMap<_, _> = compiled.into_iter().collect();

    assert_eq!(map.get(X_CONTENT_TYPE_OPTIONS).unwrap(), "nosniff");
    assert_eq!(map.get(X_FRAME_OPTIONS).unwrap(), "SAMEORIGIN");
    assert_eq!(
        map.get(STRICT_TRANSPORT_SECURITY).unwrap(),
        "max-age=31536000; includeSubDomains"
    );
    assert_eq!(map.get(X_XSS_PROTECTION).unwrap(), "0");
    assert_eq!(
        map.get(REFERRER_POLICY).unwrap(),
        "strict-origin-when-cross-origin"
    );

    // 2. OWASP
    let owasp = SecurityHeadersConfig::from_preset(SecurityPreset::Owasp);
    let owasp_compiled = owasp.compile_headers();
    let owasp_map: std::collections::HashMap<_, _> = owasp_compiled.into_iter().collect();

    assert_eq!(owasp_map.get(X_FRAME_OPTIONS).unwrap(), "DENY");
    assert_eq!(owasp_map.get(REFERRER_POLICY).unwrap(), "no-referrer");
    assert_eq!(
        owasp_map.get(CROSS_ORIGIN_EMBEDDER_POLICY).unwrap(),
        "require-corp"
    );
    assert_eq!(
        owasp_map.get(CROSS_ORIGIN_OPENER_POLICY).unwrap(),
        "same-origin"
    );
    assert_eq!(
        owasp_map.get(CROSS_ORIGIN_RESOURCE_POLICY).unwrap(),
        "same-origin"
    );
    assert_eq!(owasp_map.get(ORIGIN_AGENT_CLUSTER).unwrap(), "?1");
    assert_eq!(owasp_map.get(X_DOWNLOAD_OPTIONS).unwrap(), "noopen");
    assert_eq!(
        owasp_map.get(X_PERMITTED_CROSS_DOMAIN_POLICIES).unwrap(),
        "none"
    );
    assert!(owasp_map.contains_key(PERMISSIONS_POLICY));

    // 3. Empty
    let empty = SecurityHeadersConfig::from_preset(SecurityPreset::Empty);
    assert!(empty.compile_headers().is_empty());
}

#[test]
fn test_security_headers_builder_and_application() {
    let config = SecurityHeadersConfig::builder()
        .preset(SecurityPreset::Empty)
        .frame_guard(Some(FrameAction::Deny))
        .hsts(Some(HstsConfig::default()))
        .no_sniff(true)
        .referrer_policy(Some(ReferrerPolicy::SameOrigin))
        .permissions_policy(Some("camera=()".to_string()))
        .coep(Some("require-corp".to_string()))
        .coop(Some("same-origin".to_string()))
        .corp(Some("same-origin".to_string()))
        .origin_agent_cluster(true)
        .custom_header("x-custom-security", "enforced")
        .build();

    let mut response = Response::ok();
    config.apply(&mut response);

    assert_eq!(response.headers().get(X_FRAME_OPTIONS).unwrap(), "DENY");
    assert_eq!(
        response.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(
        response.headers().get("x-custom-security").unwrap(),
        "enforced"
    );

    // Non-destructive check: if response already has header, it is preserved
    let mut pre_existing = Response::ok();
    pre_existing.set_header(HeaderName::new(X_FRAME_OPTIONS), "SAMEORIGIN");
    config.apply(&mut pre_existing);
    assert_eq!(
        pre_existing.headers().get(X_FRAME_OPTIONS).unwrap(),
        "SAMEORIGIN"
    );

    // Overwrite existing check:
    let overwrite_config = SecurityHeadersConfig::builder()
        .preset(SecurityPreset::Empty)
        .frame_guard(Some(FrameAction::Deny))
        .overwrite_existing(true)
        .build();
    overwrite_config.apply(&mut pre_existing);
    assert_eq!(pre_existing.headers().get(X_FRAME_OPTIONS).unwrap(), "DENY");
}

// -----------------------------------------------------------------------------
// 4. Content Security Policy (CSP) & Nonces
// -----------------------------------------------------------------------------

#[test]
fn test_csp_nonce_generation_and_traits() {
    let nonce = CspNonce::generate().expect("nonce generation must succeed");
    assert_eq!(nonce.as_str().len(), 24); // 16 bytes in base64 = 24 chars
    assert_eq!(nonce.clone(), nonce);
    let inner = nonce.clone().into_inner();
    assert_eq!(inner, nonce.as_str());
    assert_eq!(
        CspNonce::new("custom_nonce".to_string()).as_str(),
        "custom_nonce"
    );
}

#[test]
fn test_csp_directive_builder_and_formatting() {
    let csp = CspConfig::builder()
        .default_src(vec!["'self'"])
        .script_src(vec!["'self'", "@nonce", "https://trusted.cdn.com"])
        .style_src(vec!["'self'", "'unsafe-inline'"])
        .img_src(vec!["'self'", "data:"])
        .font_src(vec!["'self'", "https://fonts.gstatic.com"])
        .connect_src(vec!["'self'", "https://api.example.com"])
        .object_src(vec!["'none'"])
        .frame_ancestors(vec!["'none'"])
        .form_action(vec!["'self'"])
        .base_uri(vec!["'self'"])
        .upgrade_insecure_requests()
        .block_all_mixed_content()
        .report_only(false)
        .build();

    let nonce = CspNonce::new("secret_nonce_123".to_string());
    let formatted = csp.format_header(Some(&nonce));

    assert!(formatted.contains("default-src 'self'"));
    assert!(
        formatted.contains("script-src 'self' 'nonce-secret_nonce_123' https://trusted.cdn.com")
    );
    assert!(formatted.contains("style-src 'self' 'unsafe-inline'"));
    assert!(formatted.contains("img-src 'self' data:"));
    assert!(formatted.contains("object-src 'none'"));
    assert!(formatted.contains("upgrade-insecure-requests"));
    assert!(formatted.contains("block-all-mixed-content"));

    // Fallback when no active nonce is provided
    let fallback_formatted = csp.format_header(None);
    assert!(fallback_formatted.contains("script-src 'self' 'self' https://trusted.cdn.com"));

    // Report-Only header name check
    let report_only_csp = CspConfig::builder().report_only(true).build();
    assert_eq!(
        report_only_csp.header_name(),
        CONTENT_SECURITY_POLICY_REPORT_ONLY
    );
    assert_eq!(csp.header_name(), CONTENT_SECURITY_POLICY);
}

#[test]
fn test_csp_application_and_non_destructive() {
    let csp = CspConfig::builder().default_src(vec!["'self'"]).build();

    let mut response = Response::ok();
    csp.apply(&mut response, None);
    assert_eq!(
        response.headers().get(CONTENT_SECURITY_POLICY).unwrap(),
        "default-src 'self'"
    );

    // Non-destructive: handler set its own policy
    let mut custom_response = Response::ok();
    custom_response.set_header(
        HeaderName::new(CONTENT_SECURITY_POLICY),
        "default-src 'none'",
    );
    csp.apply(&mut custom_response, None);
    assert_eq!(
        custom_response
            .headers()
            .get(CONTENT_SECURITY_POLICY)
            .unwrap(),
        "default-src 'none'"
    );

    // Overwrite when enabled:
    let overwrite_csp = CspConfig::builder()
        .default_src(vec!["'self'"])
        .overwrite_existing(true)
        .build();
    overwrite_csp.apply(&mut custom_response, None);
    assert_eq!(
        custom_response
            .headers()
            .get(CONTENT_SECURITY_POLICY)
            .unwrap(),
        "default-src 'self'"
    );

    // Disabled check
    let disabled_csp = CspConfig::builder().enabled(false).build();
    let mut resp = Response::ok();
    disabled_csp.apply(&mut resp, None);
    assert!(resp.headers().get(CONTENT_SECURITY_POLICY).is_none());
}

// -----------------------------------------------------------------------------
// 5. CSRF Token Lifecycle, Masking, and Verification
// -----------------------------------------------------------------------------

#[test]
fn test_csrf_token_types_and_traits() {
    let token = CsrfToken::new("test_token_123".to_string());
    assert_eq!(token.as_str(), "test_token_123");
    assert_eq!(token.to_string(), "test_token_123");
    assert_eq!(token.clone(), token);
    assert!(format!("{token:?}").contains("[PROTECTED]"));
    assert_eq!(token.into_inner(), "test_token_123");
}

#[test]
fn test_csrf_token_generation_and_masked_verification() {
    let secret = Key::generate().expect("key generation must succeed");

    // Generate two distinct tokens from the same secret (BREACH mitigation)
    let token1 = generate_csrf_token(&secret).expect("token 1 must generate");
    let token2 = generate_csrf_token(&secret).expect("token 2 must generate");

    assert_ne!(
        token1.as_str(),
        token2.as_str(),
        "Salted tokens must be distinct across calls"
    );

    // Both must verify successfully against the secret
    assert!(verify_csrf_token(&secret, token1.as_str()));
    assert!(verify_csrf_token(&secret, token2.as_str()));

    // Different secret must fail verification
    let wrong_secret = Key::generate().expect("wrong key must generate");
    assert!(!verify_csrf_token(&wrong_secret, token1.as_str()));

    // Tampered token must fail verification
    let mut tampered = token1.as_str().to_string();
    tampered.replace_range(
        10..11,
        if tampered.as_bytes()[10] == b'a' {
            "b"
        } else {
            "a"
        },
    );
    assert!(!verify_csrf_token(&secret, &tampered));

    // Invalid base64 or wrong length must fail verification
    assert!(!verify_csrf_token(&secret, "not-valid-base64!"));
    assert!(!verify_csrf_token(&secret, "dG9vLXNob3J0")); // too short
}

#[test]
fn test_csrf_route_exemptions() {
    let config = CsrfConfig::builder()
        .except_route("/api/webhooks/*")
        .except_route("/stripe*")
        .except_route("/health")
        .except_predicate(|ctx| ctx.request.headers().get("x-bypass-csrf").is_some())
        .build();

    // Wildcard /*
    assert!(config.is_exempt(&make_context(Method::Post, "/api/webhooks/github")));
    assert!(config.is_exempt(&make_context(Method::Post, "/api/webhooks")));
    assert!(!config.is_exempt(&make_context(Method::Post, "/api/users")));

    // Wildcard *
    assert!(config.is_exempt(&make_context(Method::Post, "/stripe/charge")));
    assert!(config.is_exempt(&make_context(Method::Post, "/stripe")));

    // Exact match
    assert!(config.is_exempt(&make_context(Method::Post, "/health")));
    assert!(!config.is_exempt(&make_context(Method::Post, "/healthy")));

    // Predicate exemption
    let bypassed_ctx =
        make_context_with_headers(Method::Post, "/checkout", &[("x-bypass-csrf", "true")]);
    assert!(config.is_exempt(&bypassed_ctx));

    let regular_ctx = make_context(Method::Post, "/checkout");
    assert!(!config.is_exempt(&regular_ctx));
}

#[test]
fn test_csrf_token_extraction_sources() {
    let secret = Key::generate().expect("secret");
    let valid_token = generate_csrf_token(&secret).expect("token");

    // 1. Header: X-CSRF-TOKEN
    let ctx_header1 = make_context_with_headers(
        Method::Post,
        "/submit",
        &[(X_CSRF_TOKEN, valid_token.as_str())],
    );
    assert_eq!(
        extract_csrf_token(&ctx_header1).unwrap(),
        valid_token.as_str()
    );

    // 2. Header: X-XSRF-TOKEN
    let ctx_header2 = make_context_with_headers(
        Method::Post,
        "/submit",
        &[(X_XSRF_TOKEN, valid_token.as_str())],
    );
    assert_eq!(
        extract_csrf_token(&ctx_header2).unwrap(),
        valid_token.as_str()
    );

    // 3. Query string: _csrf
    let ctx_query1 = make_context(
        Method::Post,
        &format!("/submit?_csrf={}", valid_token.as_str()),
    );
    assert_eq!(
        extract_csrf_token(&ctx_query1).unwrap(),
        valid_token.as_str()
    );

    // 4. Query string: _token
    let ctx_query2 = make_context(
        Method::Post,
        &format!("/submit?_token={}", valid_token.as_str()),
    );
    assert_eq!(
        extract_csrf_token(&ctx_query2).unwrap(),
        valid_token.as_str()
    );

    // 5. Form URL-encoded body: _csrf
    let form_body = format!("foo=bar&_csrf={}&baz=qux", valid_token.as_str());
    let ctx_form = make_context_with_body(
        Method::Post,
        "/submit",
        form_body.as_bytes(),
        "application/x-www-form-urlencoded",
    );
    assert_eq!(extract_csrf_token(&ctx_form).unwrap(), valid_token.as_str());

    // 6. JSON body: _token
    let json_body = format!(
        r#"{{"user": "alice", "_token": "{}"}}"#,
        valid_token.as_str()
    );
    let ctx_json = make_context_with_body(
        Method::Post,
        "/submit",
        json_body.as_bytes(),
        "application/json",
    );
    assert_eq!(extract_csrf_token(&ctx_json).unwrap(), valid_token.as_str());

    // 7. JSON body: _csrf
    let json_body_csrf = format!(
        r#"{{"user": "alice", "_csrf": "{}"}}"#,
        valid_token.as_str()
    );
    let ctx_json_csrf = make_context_with_body(
        Method::Post,
        "/submit",
        json_body_csrf.as_bytes(),
        "application/json",
    );
    assert_eq!(
        extract_csrf_token(&ctx_json_csrf).unwrap(),
        valid_token.as_str()
    );

    // 8. Form URL-encoded body: _token
    let form_body_token = format!("foo=bar&_token={}&baz=qux", valid_token.as_str());
    let ctx_form_token = make_context_with_body(
        Method::Post,
        "/submit",
        form_body_token.as_bytes(),
        "application/x-www-form-urlencoded",
    );
    assert_eq!(
        extract_csrf_token(&ctx_form_token).unwrap(),
        valid_token.as_str()
    );

    // 9. JSON body edge cases: invalid syntax, missing token, empty token, non-string token
    let ctx_invalid_json = make_context_with_body(
        Method::Post,
        "/submit",
        b"{ not valid json syntax",
        "application/json",
    );
    assert!(extract_csrf_token(&ctx_invalid_json).is_none());

    let ctx_no_token_json = make_context_with_body(
        Method::Post,
        "/submit",
        br#"{"username": "bob"}"#,
        "application/json",
    );
    assert!(extract_csrf_token(&ctx_no_token_json).is_none());

    let ctx_empty_token_json = make_context_with_body(
        Method::Post,
        "/submit",
        br#"{"_csrf": ""}"#,
        "application/json",
    );
    assert!(extract_csrf_token(&ctx_empty_token_json).is_none());

    let ctx_num_token_json = make_context_with_body(
        Method::Post,
        "/submit",
        br#"{"_csrf": 12345}"#,
        "application/json",
    );
    assert!(extract_csrf_token(&ctx_num_token_json).is_none());

    // 10. Non-UTF-8 body bytes
    let ctx_binary = make_context_with_body(
        Method::Post,
        "/submit",
        [0xFF, 0xFE, 0xFD],
        "application/octet-stream",
    );
    assert!(extract_csrf_token(&ctx_binary).is_none());

    // 11. Request extension
    let mut ctx_ext = make_context(Method::Post, "/submit");
    ctx_ext.extensions.insert(valid_token.clone());
    assert_eq!(extract_csrf_token(&ctx_ext).unwrap(), valid_token.as_str());

    // 12. Missing token returns None
    let ctx_empty = make_context(Method::Post, "/submit");
    assert!(extract_csrf_token(&ctx_empty).is_none());
}

#[test]
fn test_csrf_secret_resolution_with_and_without_session() {
    let config = CsrfConfig::default();

    // 1. With active SessionContext
    let mut ctx_session = make_context(Method::Get, "/");
    let session = Session::new_fresh(SessionId::generate().unwrap(), 3600);
    ctx_session.set_session(session);

    let resolved_key1 = config
        .resolve_or_init_secret(&mut ctx_session)
        .expect("must init secret");
    // Second resolution must reuse the same secret
    let resolved_key2 = config
        .resolve_or_init_secret(&mut ctx_session)
        .expect("must retrieve secret");
    assert_eq!(resolved_key1, resolved_key2);

    // 2. Without active session (fallback to extensions)
    let mut ctx_fallback = make_context(Method::Get, "/");
    let fallback_key1 = config
        .resolve_or_init_secret(&mut ctx_fallback)
        .expect("must generate fallback");
    let fallback_key2 = config
        .resolve_or_init_secret(&mut ctx_fallback)
        .expect("must reuse fallback");
    assert_eq!(fallback_key1, fallback_key2);
}

// -----------------------------------------------------------------------------
// 6. Standalone Middlewares Pipeline Verification
// -----------------------------------------------------------------------------

#[test]
fn test_security_headers_middleware_pipeline() {
    let middleware = SecurityHeadersMiddleware::default();
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Get, "/");
    let terminal = from_terminal(|_ctx: &mut HttpContext| {
        Box::pin(async {
            let mut response = Response::ok();
            response.text("Hello World");
            Ok(response)
        })
    });

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    assert_eq!(
        response.headers().get(X_CONTENT_TYPE_OPTIONS).unwrap(),
        "nosniff"
    );
    assert_eq!(
        response.headers().get(X_FRAME_OPTIONS).unwrap(),
        "SAMEORIGIN"
    );
    assert_eq!(
        response.headers().get(STRICT_TRANSPORT_SECURITY).unwrap(),
        "max-age=31536000; includeSubDomains"
    );
}

#[test]
fn test_csp_middleware_pipeline_with_nonce() {
    let config = CspConfig::builder()
        .script_src(vec!["'self'", "@nonce"])
        .generate_nonce(true)
        .build();
    let middleware = CspMiddleware::new(config);
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Get, "/");
    let terminal = from_terminal(|ctx: &mut HttpContext| {
        Box::pin(async {
            // Assert nonce is available in extensions
            let nonce = ctx
                .extensions
                .get::<CspNonce>()
                .expect("nonce in extensions");
            assert!(!nonce.as_str().is_empty());
            Ok(Response::ok())
        })
    });

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let csp_header = response
        .headers()
        .get(CONTENT_SECURITY_POLICY)
        .expect("CSP header");

    assert!(csp_header.contains("script-src 'self' 'nonce-"));
}

#[test]
fn test_csrf_middleware_safe_method_and_cookie() {
    let middleware = CsrfMiddleware::default();
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Get, "/");
    let terminal = from_terminal(|ctx: &mut HttpContext| {
        Box::pin(async {
            // CSRF token must be placed in extensions
            assert!(ctx.extensions.get::<CsrfToken>().is_some());
            Ok(Response::ok())
        })
    });

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);

    // Outgoing cookie jar must have XSRF-TOKEN
    let xsrf_cookie = ctx
        .cookie_jar()
        .get_staged(DEFAULT_XSRF_COOKIE_NAME)
        .expect("XSRF-TOKEN staged");
    assert!(!xsrf_cookie.http_only());
    assert_eq!(xsrf_cookie.same_site(), Some(SameSite::Lax));
}

#[test]
fn test_csrf_middleware_mutating_method_rejection() {
    let middleware = CsrfMiddleware::default();
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Post, "/create");
    let called = Arc::new(AtomicBool::new(false));
    let called_clone = Arc::clone(&called);

    let terminal = from_terminal(move |_ctx: &mut HttpContext| {
        let flag = Arc::clone(&called_clone);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            Ok(Response::ok())
        })
    });

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();

    // Downstream handler must NEVER be invoked on invalid CSRF
    assert!(!called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
}

#[test]
fn test_csrf_middleware_failure_mode_error() {
    let config = CsrfConfig::builder()
        .failure_mode(CsrfFailureMode::Error)
        .build();
    let middleware = CsrfMiddleware::new(config);
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Post, "/delete");
    let terminal = from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));

    let result = block_on(pipeline.run(&mut ctx, terminal));
    assert!(result.is_err());
    match result.unwrap_err() {
        PipelineError::Execution(RouterError::InvalidSignature) => {}
        other => panic!("Unexpected error: {other:?}"),
    }
}

#[test]
fn test_csrf_middleware_valid_token_pass_through() {
    let config = CsrfConfig::default();
    let middleware = CsrfMiddleware::new(config.clone());
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Post, "/update");
    // Pre-initialize secret and token
    let secret = config.resolve_or_init_secret(&mut ctx).unwrap();
    let token = config.generate_token(&secret).unwrap();
    ctx.request
        .headers_mut()
        .insert(X_CSRF_TOKEN, token.as_str());

    let called = Arc::new(AtomicBool::new(false));
    let called_clone = Arc::clone(&called);

    let terminal = from_terminal(move |_ctx: &mut HttpContext| {
        let flag = Arc::clone(&called_clone);
        Box::pin(async move {
            flag.store(true, Ordering::SeqCst);
            let mut resp = Response::ok();
            resp.text("Updated");
            Ok(resp)
        })
    });

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(called.load(Ordering::SeqCst));
    assert_eq!(response.status(), StatusCode::OK);
}

// -----------------------------------------------------------------------------
// 7. Unified ShieldMiddleware Full Lifecycle Verification
// -----------------------------------------------------------------------------

#[test]
fn test_unified_shield_middleware_full_stack() {
    let shield = ShieldMiddleware::new(
        ShieldConfig::builder()
            .preset(SecurityPreset::SecureDefaults)
            .csp(
                CspConfig::builder()
                    .script_src(vec!["'self'", "@nonce"])
                    .generate_nonce(true)
                    .build(),
            )
            .csrf(CsrfConfig::default())
            .build(),
    );

    let pipeline = Pipeline::new().pipe(shield);

    // 1. GET Request
    let mut get_ctx = make_context(Method::Get, "/profile");
    let terminal = from_terminal(|ctx: &mut HttpContext| {
        Box::pin(async {
            assert!(ctx.extensions.get::<CsrfToken>().is_some());
            assert!(ctx.extensions.get::<CspNonce>().is_some());
            let mut resp = Response::ok();
            resp.text("Profile Page");
            Ok(resp)
        })
    });

    let get_resp = block_on(pipeline.run(&mut get_ctx, terminal)).unwrap();
    assert_eq!(get_resp.status(), StatusCode::OK);
    assert!(get_resp.headers().get(X_CONTENT_TYPE_OPTIONS).is_some());
    assert!(get_resp.headers().get(CONTENT_SECURITY_POLICY).is_some());

    // 2. Mutating POST without CSRF
    let mut post_ctx = make_context(Method::Post, "/profile");
    let post_terminal =
        from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));
    let post_resp = block_on(pipeline.run(&mut post_ctx, post_terminal)).unwrap();
    assert_eq!(post_resp.status(), StatusCode::FORBIDDEN);

    // 3. Non-destructive check with custom handler headers
    let custom_terminal = from_terminal(|_ctx: &mut HttpContext| {
        Box::pin(async {
            let mut resp = Response::ok();
            resp.set_header(HeaderName::new(X_FRAME_OPTIONS), "DENY");
            Ok(resp)
        })
    });

    let mut custom_ctx = make_context(Method::Get, "/admin");
    let custom_resp = block_on(pipeline.run(&mut custom_ctx, custom_terminal)).unwrap();
    assert_eq!(custom_resp.headers().get(X_FRAME_OPTIONS).unwrap(), "DENY");
}

#[test]
fn test_shield_builders_and_getters() {
    let shield_cfg = ShieldConfig::builder()
        .preset(SecurityPreset::Owasp)
        .headers(SecurityHeadersConfig::default())
        .csp(CspConfig::default())
        .csrf(CsrfConfig::default())
        .build();

    let middleware = ShieldMiddleware::new(shield_cfg);
    assert!(middleware.config().headers.is_some());
    assert!(middleware.config().csp.is_some());
    assert!(middleware.config().csrf.is_some());
}

#[test]
fn test_csrf_builder_all_setters() {
    let config = CsrfConfig::builder()
        .enabled(true)
        .session_key("custom.csrf.key")
        .xsrf_cookie_name("CUSTOM-XSRF")
        .enable_xsrf_cookie(false)
        .cookie_path("/app")
        .cookie_same_site(SameSite::Strict)
        .cookie_secure(true)
        .failure_mode(CsrfFailureMode::Error)
        .except_routes(vec!["/webhook1", "/webhook2"])
        .except_predicate(|_ctx| false)
        .build();

    assert!(config.enabled);
    assert_eq!(config.session_key, "custom.csrf.key");
    assert_eq!(config.xsrf_cookie_name, "CUSTOM-XSRF");
    assert!(!config.enable_xsrf_cookie);
    assert_eq!(config.cookie_path, "/app");
    assert_eq!(config.cookie_same_site, SameSite::Strict);
    assert!(config.cookie_secure);
    assert_eq!(config.failure_mode, CsrfFailureMode::Error);
    assert_eq!(config.except_routes.len(), 2);
    assert!(config.except_predicate.is_some());
    assert!(format!("{config:?}").contains("CsrfConfig"));
}

#[test]
fn test_csrf_disabled_middleware() {
    let config = CsrfConfig::builder().enabled(false).build();
    let middleware = CsrfMiddleware::new(config);
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Post, "/unprotected");
    let terminal = from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_csp_disabled_middleware() {
    let config = CspConfig::builder().enabled(false).build();
    let middleware = CspMiddleware::new(config);
    let pipeline = Pipeline::new().pipe(middleware);

    let mut ctx = make_context(Method::Get, "/");
    let terminal = from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert!(response.headers().get(CONTENT_SECURITY_POLICY).is_none());
}

#[test]
fn test_middleware_builders_and_getters() {
    let sec_mw = SecurityHeadersMiddleware::from_builder(SecurityHeadersConfig::builder());
    assert_eq!(sec_mw.config().preset, SecurityPreset::SecureDefaults);

    let csp_mw = CspMiddleware::from_builder(CspConfig::builder());
    assert!(csp_mw.config().enabled);
    let default_csp_mw = CspMiddleware::default();
    assert!(default_csp_mw.config().enabled);

    let csrf_mw = CsrfMiddleware::from_builder(CsrfConfig::builder());
    assert!(csrf_mw.config().enabled);
    assert_eq!(csrf_mw.config().session_key, DEFAULT_CSRF_SESSION_KEY);

    let default_shield = ShieldMiddleware::default();
    assert!(default_shield.config().headers.is_some());
}

#[test]
fn test_shield_middleware_failure_mode_error() {
    let shield = ShieldMiddleware::new(
        ShieldConfig::builder()
            .csrf(
                CsrfConfig::builder()
                    .failure_mode(CsrfFailureMode::Error)
                    .build(),
            )
            .build(),
    );
    let pipeline = Pipeline::new().pipe(shield);

    let mut ctx = make_context(Method::Post, "/sensitive");
    let terminal = from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));

    let result = block_on(pipeline.run(&mut ctx, terminal));
    assert!(result.is_err());
}

#[test]
fn test_shield_middleware_exempt_route() {
    let shield = ShieldMiddleware::new(
        ShieldConfig::builder()
            .csrf(CsrfConfig::builder().except_route("/api/hook").build())
            .build(),
    );
    let pipeline = Pipeline::new().pipe(shield);

    let mut ctx = make_context(Method::Post, "/api/hook");
    let terminal = from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

#[test]
fn test_csrf_attach_cookie_disabled() {
    let config = CsrfConfig::builder().enable_xsrf_cookie(false).build();
    let secret = Key::generate().unwrap();
    let token = generate_csrf_token(&secret).unwrap();
    let mut ctx = make_context(Method::Get, "/");

    config.attach_xsrf_cookie(&mut ctx, &token);
    assert!(
        ctx.cookie_jar()
            .get_staged(DEFAULT_XSRF_COOKIE_NAME)
            .is_none()
    );

    // Invalid cookie name failure mode
    let invalid_config = CsrfConfig::builder()
        .xsrf_cookie_name("invalid cookie name with spaces")
        .build();
    let mut ctx_invalid = make_context(Method::Get, "/");
    invalid_config.attach_xsrf_cookie(&mut ctx_invalid, &token);
    assert!(
        ctx_invalid
            .cookie_jar()
            .get_staged("invalid cookie name with spaces")
            .is_none()
    );
}

#[test]
fn test_csrf_extract_token_empty_and_whitespace() {
    let ctx = make_context_with_headers(
        Method::Post,
        "/",
        &[(X_CSRF_TOKEN, "   "), (X_XSRF_TOKEN, "")],
    );
    assert!(extract_csrf_token(&ctx).is_none());

    let ctx_query = make_context(Method::Post, "/?_csrf=&_token=");
    assert!(extract_csrf_token(&ctx_query).is_none());

    let ctx_form = make_context_with_body(
        Method::Post,
        "/",
        b"_csrf=&_token=",
        "application/x-www-form-urlencoded",
    );
    assert!(extract_csrf_token(&ctx_form).is_none());

    let ctx_json_empty = make_context_with_body(
        Method::Post,
        "/",
        br#"{"_csrf": "", "_token": ""}"#,
        "application/json",
    );
    assert!(extract_csrf_token(&ctx_json_empty).is_none());
}

#[test]
fn test_security_headers_all_builder_setters() {
    let config = SecurityHeadersConfig::builder()
        .preset(SecurityPreset::Empty)
        .frame_guard(Some(FrameAction::SameOrigin))
        .hsts(Some(HstsConfig::default()))
        .no_sniff(true)
        .referrer_policy(Some(ReferrerPolicy::NoReferrer))
        .permissions_policy(Some("usb=()".to_string()))
        .coep(Some("require-corp".to_string()))
        .coop(Some("same-origin".to_string()))
        .corp(Some("same-origin".to_string()))
        .origin_agent_cluster(true)
        .custom_header("x-foo", "bar")
        .overwrite_existing(true)
        .build();

    let compiled = config.compile_headers();
    let map: std::collections::HashMap<_, _> = compiled.into_iter().collect();

    assert_eq!(map.get(X_FRAME_OPTIONS).unwrap(), "SAMEORIGIN");
    assert_eq!(map.get(REFERRER_POLICY).unwrap(), "no-referrer");
    assert_eq!(map.get(PERMISSIONS_POLICY).unwrap(), "usb=()");
    assert_eq!(
        map.get(CROSS_ORIGIN_EMBEDDER_POLICY).unwrap(),
        "require-corp"
    );
    assert_eq!(map.get(CROSS_ORIGIN_OPENER_POLICY).unwrap(), "same-origin");
    assert_eq!(
        map.get(CROSS_ORIGIN_RESOURCE_POLICY).unwrap(),
        "same-origin"
    );
    assert_eq!(map.get(ORIGIN_AGENT_CLUSTER).unwrap(), "?1");
    assert_eq!(map.get("x-foo").unwrap(), "bar");
}

#[test]
fn test_csp_all_builder_setters() {
    let config = CspConfig::builder()
        .enabled(true)
        .directive("worker-src", vec!["'self'"])
        .font_src(vec!["'self'"])
        .connect_src(vec!["'self'"])
        .frame_ancestors(vec!["'none'"])
        .form_action(vec!["'self'"])
        .base_uri(vec!["'self'"])
        .upgrade_insecure_requests()
        .block_all_mixed_content()
        .build();

    let header = config.format_header(None);
    assert!(header.contains("worker-src 'self'"));
    assert!(header.contains("font-src 'self'"));
    assert!(header.contains("connect-src 'self'"));
    assert!(header.contains("frame-ancestors 'none'"));
    assert!(header.contains("form-action 'self'"));
    assert!(header.contains("base-uri 'self'"));
}

#[test]
fn test_unified_shield_post_with_valid_token() {
    let shield = ShieldMiddleware::new(ShieldConfig::default());
    let pipeline = Pipeline::new().pipe(shield);

    let mut ctx = make_context(Method::Post, "/form");
    let secret = CsrfConfig::default()
        .resolve_or_init_secret(&mut ctx)
        .unwrap();
    let token = generate_csrf_token(&secret).unwrap();
    ctx.request
        .headers_mut()
        .insert(X_CSRF_TOKEN, token.as_str());

    let terminal = from_terminal(|_ctx: &mut HttpContext| {
        Box::pin(async {
            let mut resp = Response::ok();
            resp.set_header(HeaderName::SET_COOKIE, "session_id=abc; Path=/");
            Ok(resp)
        })
    });

    let response = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert!(response.headers().get(X_CONTENT_TYPE_OPTIONS).is_some());
}

#[test]
fn test_clear_site_data_configuration() {
    let mut config = SecurityHeadersConfig::from_preset(SecurityPreset::Empty);
    config.clear_site_data = Some("\"cache\",\"cookies\"".to_string());
    let mut response = Response::ok();
    config.apply(&mut response);

    assert_eq!(
        response.headers().get(CLEAR_SITE_DATA).unwrap(),
        "\"cache\",\"cookies\""
    );
}

#[test]
fn test_standard_base64_padded_token_verification() {
    let secret = Key::generate().unwrap();
    let signer = crypto::hmac::HmacSigner::new(secret.clone());
    let salt = [42u8; 16];
    let sig = signer.sign(&salt);
    let mut combined = [0u8; 48];
    combined[0..16].copy_from_slice(&salt);
    combined[16..48].copy_from_slice(&sig);

    let standard_b64 = BASE64_STANDARD.encode(combined);
    assert!(verify_csrf_token(&secret, &standard_b64));
}

#[test]
fn test_shield_none_subguards_and_csp_no_nonce() {
    // Shield with all subguards set to None
    let bare_shield = ShieldMiddleware::new(
        ShieldConfig::builder()
            .headers(None)
            .csp(None)
            .csrf(None)
            .build(),
    );
    let pipeline = Pipeline::new().pipe(bare_shield);
    let mut ctx = make_context(Method::Get, "/");
    let terminal = from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));
    let resp = block_on(pipeline.run(&mut ctx, terminal)).unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    // CspMiddleware with generate_nonce = false
    let csp_no_nonce = CspMiddleware::new(
        CspConfig::builder()
            .default_src(vec!["'self'"])
            .generate_nonce(false)
            .build(),
    );
    let csp_pipeline = Pipeline::new().pipe(csp_no_nonce);
    let mut csp_ctx = make_context(Method::Get, "/");
    let csp_terminal = from_terminal(|ctx: &mut HttpContext| {
        Box::pin(async {
            assert!(ctx.extensions.get::<CspNonce>().is_none());
            Ok(Response::ok())
        })
    });
    let csp_resp = block_on(csp_pipeline.run(&mut csp_ctx, csp_terminal)).unwrap();
    assert_eq!(
        csp_resp.headers().get(CONTENT_SECURITY_POLICY).unwrap(),
        "default-src 'self'"
    );

    // CsrfMiddleware on exempt route for POST
    let csrf_exempt =
        CsrfMiddleware::new(CsrfConfig::builder().except_route("/exempt-path").build());
    let csrf_pipeline = Pipeline::new().pipe(csrf_exempt);
    let mut csrf_ctx = make_context(Method::Post, "/exempt-path");
    let csrf_terminal =
        from_terminal(|_ctx: &mut HttpContext| Box::pin(async { Ok(Response::ok()) }));
    let csrf_resp = block_on(csrf_pipeline.run(&mut csrf_ctx, csrf_terminal)).unwrap();
    assert_eq!(csrf_resp.status(), StatusCode::OK);
}
