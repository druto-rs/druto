//! Integration tests for BodyParserMiddleware executing within an onion pipeline.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use bodyparser::{
    BodyParserConfig, BodyParserMiddleware, FormFields, PayloadLimits, UploadedFiles,
};
use matcher::Method;
use pipeline::{Pipeline, PipelineError, from_terminal};
use router::{HeaderName, HttpContext, Request, Response, RouterError, StatusCode};

fn execute_pipeline(
    middleware: BodyParserMiddleware,
    ctx: &mut HttpContext,
    called: Arc<AtomicBool>,
) -> Result<Response, PipelineError<RouterError>> {
    let pipeline = Pipeline::new().pipe(middleware);
    let terminal = from_terminal(move |ctx: &mut HttpContext| {
        called.store(true, Ordering::SeqCst);
        ctx.response.set_status(StatusCode::OK);
        let resp = ctx.response.clone();
        Box::pin(async move { Ok(resp) })
    });

    // Run asynchronously via standard block_on or tokio
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(pipeline.run(ctx, terminal))
}

#[test]
fn test_middleware_json_payload_success() {
    let request = Request::builder()
        .method(Method::Post)
        .path("/api/users")
        .header(HeaderName::CONTENT_TYPE, "application/json")
        .body(br#"{"username":"test_user","active":true}"#.to_vec())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let middleware = BodyParserMiddleware::default_config();
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_middleware_json_size_limit_exceeded() {
    let request = Request::builder()
        .method(Method::Post)
        .path("/api/users")
        .header(HeaderName::CONTENT_TYPE, "application/json")
        .body(br#"{"long":"payload_exceeding_limit"}"#.to_vec())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let limits = PayloadLimits::builder().max_body_size(10).build();
    let middleware = BodyParserMiddleware::with_limits(limits);
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_err());
    assert!(!called.load(Ordering::SeqCst));

    match result.unwrap_err() {
        PipelineError::Execution(RouterError::PayloadTooLarge {
            max_bytes,
            received_bytes,
        }) => {
            assert_eq!(max_bytes, 10);
            assert_eq!(received_bytes, 34);
        }
        other => panic!("Expected PayloadTooLarge, got: {other:?}"),
    }
}

#[test]
fn test_middleware_form_payload_populates_extensions() {
    let request = Request::builder()
        .method(Method::Post)
        .path("/login")
        .header(
            HeaderName::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(b"username=admin&password=secret&remember=true".to_vec())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let middleware = BodyParserMiddleware::default_config();
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));

    let form_fields = ctx.extensions.get::<FormFields>().unwrap();
    assert_eq!(form_fields.get("username"), Some("admin"));
    assert_eq!(form_fields.get("password"), Some("secret"));
    assert_eq!(form_fields.get("remember"), Some("true"));
}

#[test]
fn test_middleware_multipart_payload_populates_extensions() {
    let boundary = "FORM_BOUNDARY";
    let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A];

    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"title\"\r\n\r\nMy Document\r\n",
    );
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"attachment\"; filename=\"image.png\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(&png_header);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let request = Request::builder()
        .method(Method::Post)
        .path("/upload")
        .header(
            HeaderName::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body)
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let middleware = BodyParserMiddleware::default_config();
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));

    // Verify text fields in extensions
    let form_fields = ctx.extensions.get::<FormFields>().unwrap();
    assert_eq!(form_fields.get("title"), Some("My Document"));

    // Verify uploaded files in extensions
    let uploaded_files = ctx.extensions.get::<UploadedFiles>().unwrap();
    assert_eq!(uploaded_files.len(), 1);

    let file = uploaded_files.file("attachment").unwrap();
    assert_eq!(file.client_filename(), "image.png");
    assert_eq!(file.detected_mime_type(), "image/png");
    assert_eq!(file.size(), 8);
    assert!(file.is_valid());
}

#[test]
fn test_middleware_manual_multipart_bypass() {
    let boundary = "MANUAL_B";
    let body = format!(
        "--{boundary}\r\nContent-Disposition: form-data; name=\"raw\"\r\n\r\nval\r\n--{boundary}--\r\n"
    );

    let request = Request::builder()
        .method(Method::Post)
        .path("/streaming/direct")
        .header(
            HeaderName::CONTENT_TYPE,
            format!("multipart/form-data; boundary={boundary}"),
        )
        .body(body.as_bytes().to_vec())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let config = BodyParserConfig::builder()
        .process_manually("/streaming/*")
        .build();
    let middleware = BodyParserMiddleware::new(config);
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));

    // When bypassed, no automatic extensions are injected
    assert!(ctx.extensions.get::<UploadedFiles>().is_none());
    assert!(ctx.extensions.get::<FormFields>().is_none());
}

#[test]
fn test_middleware_ignored_method_bypasses_parsing() {
    let request = Request::builder()
        .method(Method::Get)
        .path("/search")
        .header(HeaderName::CONTENT_TYPE, "application/json")
        .body(b"some body".to_vec())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let middleware = BodyParserMiddleware::default_config();
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));
    assert!(ctx.extensions.get::<FormFields>().is_none());
}

#[test]
fn test_middleware_empty_body_bypasses_parsing() {
    let request = Request::builder()
        .method(Method::Post)
        .path("/empty")
        .header(HeaderName::CONTENT_TYPE, "application/json")
        .body(Vec::new())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let middleware = BodyParserMiddleware::default_config();
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));
}

#[test]
fn test_middleware_query_method_parses_body() {
    let request = Request::builder()
        .method(Method::Query)
        .path("/search")
        .header(
            HeaderName::CONTENT_TYPE,
            "application/x-www-form-urlencoded",
        )
        .body(b"filter%5Bname%5D=alice".to_vec())
        .build();

    let mut ctx = HttpContext::new(request);
    let called = Arc::new(AtomicBool::new(false));

    let middleware = BodyParserMiddleware::default_config();
    let result = execute_pipeline(middleware, &mut ctx, Arc::clone(&called));

    assert!(result.is_ok());
    assert!(called.load(Ordering::SeqCst));

    let form_fields = ctx
        .extensions
        .get::<FormFields>()
        .expect("FormFields must be injected for QUERY method");
    let structured = form_fields.tree();
    assert_eq!(
        structured
            .get("filter")
            .and_then(|f| f.get("name"))
            .and_then(|v| v.as_scalar()),
        Some("alice")
    );
}
