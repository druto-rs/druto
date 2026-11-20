//! Unit and integration tests for streaming multipart boundary state machine.

use bodyparser::{
    BodyParserError, MultipartParser, PayloadLimits, extract_boundary, parse_multipart_stream,
};
use bytes::Bytes;
use futures_util::stream;

#[test]
fn test_extract_boundary() {
    assert_eq!(
        extract_boundary("multipart/form-data; boundary=---------------------------9747672998")
            .unwrap(),
        "---------------------------9747672998"
    );
    assert_eq!(
        extract_boundary("multipart/form-data; charset=utf-8; boundary=\"custom-boundary\"")
            .unwrap(),
        "custom-boundary"
    );
    assert!(extract_boundary("application/json").is_err());
    assert!(extract_boundary("multipart/form-data; boundary=\"\"").is_err());
}

#[test]
fn test_parse_simple_multipart_text_fields() {
    let boundary = "----WebKitFormBoundary7MA4YWxkTrZu0gW";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"username\"\r\n\
        \r\n\
        john_doe\r\n\
        --{boundary}\r\n\
        Content-Disposition: form-data; name=\"email\"\r\n\
        \r\n\
        john@example.com\r\n\
        --{boundary}--\r\n"
    );

    let parser = MultipartParser::new(boundary, PayloadLimits::default());
    let parsed = parser.parse_slice(body.as_bytes()).unwrap();

    assert_eq!(parsed.text("username"), Some("john_doe"));
    assert_eq!(parsed.text("email"), Some("john@example.com"));
    assert_eq!(parsed.all_files().len(), 0);
}

#[test]
fn test_parse_nested_fields_via_qs() {
    let boundary = "XYZ";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"user[profile][name]\"\r\n\
        \r\n\
        Alice\r\n\
        --{boundary}\r\n\
        Content-Disposition: form-data; name=\"user[profile][age]\"\r\n\
        \r\n\
        30\r\n\
        --{boundary}\r\n\
        Content-Disposition: form-data; name=\"tags[]\"\r\n\
        \r\n\
        admin\r\n\
        --{boundary}\r\n\
        Content-Disposition: form-data; name=\"tags[]\"\r\n\
        \r\n\
        developer\r\n\
        --{boundary}--\r\n"
    );

    let parser = MultipartParser::new(boundary, PayloadLimits::default());
    let parsed = parser.parse_slice(body.as_bytes()).unwrap();

    let structured = parsed.structured_fields().unwrap();
    let user = structured.get("user").unwrap();
    let profile = user.get("profile").unwrap();
    assert_eq!(profile.get("name").unwrap().as_scalar(), Some("Alice"));
    assert_eq!(profile.get("age").unwrap().as_scalar(), Some("30"));

    let tags = structured.get("tags").unwrap().as_array().unwrap();
    assert_eq!(tags.len(), 2);
    assert_eq!(tags[0].as_scalar(), Some("admin"));
    assert_eq!(tags[1].as_scalar(), Some("developer"));
}

#[test]
fn test_parse_file_upload_with_magic_bytes() {
    let boundary = "BOUNDARY123";
    let png_header = [0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A, 0x00, 0x01];

    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"avatar\"; filename=\"photo.png\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: image/png\r\n\r\n");
    body.extend_from_slice(&png_header);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let parser = MultipartParser::new(boundary, PayloadLimits::default());
    let parsed = parser.parse_slice(&body).unwrap();

    assert_eq!(parsed.all_files().len(), 1);
    let file = parsed.file("avatar").unwrap();
    assert_eq!(file.client_filename(), "photo.png");
    assert_eq!(file.detected_mime_type(), "image/png");
    assert_eq!(file.size(), 10);
    assert!(file.is_valid());
}

#[test]
fn test_field_count_limit_exceeded() {
    let boundary = "B";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"f1\"\r\n\r\nv1\r\n\
        --{boundary}\r\n\
        Content-Disposition: form-data; name=\"f2\"\r\n\r\nv2\r\n\
        --{boundary}\r\n\
        Content-Disposition: form-data; name=\"f3\"\r\n\r\nv3\r\n\
        --{boundary}--\r\n"
    );

    let limits = PayloadLimits::builder().max_fields_count(2).build();
    let parser = MultipartParser::new(boundary, limits);

    let err = parser.parse_slice(body.as_bytes()).unwrap_err();
    assert_eq!(err, BodyParserError::FieldCountExceeded { limit: 2 });
}

#[test]
fn test_file_size_limit_exceeded() {
    let boundary = "B";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"file\"; filename=\"test.txt\"\r\n\r\n",
    );
    body.extend_from_slice(&[b'a'; 100]);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());

    let limits = PayloadLimits::builder().max_file_size(50).build();
    let parser = MultipartParser::new(boundary, limits);

    let err = parser.parse_slice(&body).unwrap_err();
    assert_eq!(
        err,
        BodyParserError::PayloadTooLarge {
            limit: 50,
            received: 100,
        }
    );
}

#[test]
fn test_malformed_missing_boundary() {
    let parser = MultipartParser::new("B", PayloadLimits::default());
    let err = parser.parse_slice(b"no boundary here").unwrap_err();
    assert!(matches!(err, BodyParserError::MalformedMultipart(_)));
}

#[tokio::test]
async fn test_parse_multipart_stream_success() {
    let boundary = "STREAM_B";
    let body = format!(
        "--{boundary}\r\n\
        Content-Disposition: form-data; name=\"field\"\r\n\
        \r\n\
        stream_value\r\n\
        --{boundary}--\r\n"
    );

    // Split body into multiple small chunk packets to test streaming chunk ingestion
    let chunks: Vec<Result<Bytes, std::io::Error>> = body
        .as_bytes()
        .chunks(10)
        .map(|chunk| Ok(Bytes::copy_from_slice(chunk)))
        .collect();

    let stream = stream::iter(chunks);
    let parsed = parse_multipart_stream(stream, boundary, PayloadLimits::default(), None)
        .await
        .unwrap();

    assert_eq!(parsed.text("field"), Some("stream_value"));
}

#[tokio::test]
async fn test_parse_multipart_stream_fail_fast_on_limit() {
    let boundary = "STREAM_B";
    let chunk1 = Bytes::from_static(&[b'x'; 50]);
    let chunk2 = Bytes::from_static(&[b'y'; 60]);

    let stream = stream::iter(vec![
        Ok::<Bytes, std::io::Error>(chunk1),
        Ok::<Bytes, std::io::Error>(chunk2),
    ]);

    let limits = PayloadLimits::builder().max_body_size(80).build();
    let err = parse_multipart_stream(stream, boundary, limits, None)
        .await
        .unwrap_err();

    assert_eq!(
        err,
        BodyParserError::PayloadTooLarge {
            limit: 80,
            received: 110,
        }
    );
}
