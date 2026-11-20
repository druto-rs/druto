use std::error::Error;

use bodyparser::BodyParserError;
use router::{RouterError, StatusCode};

#[test]
fn test_body_parser_error_status_codes_and_display() {
    let cases = vec![
        (
            BodyParserError::PayloadTooLarge {
                limit: 1024,
                received: 2048,
            },
            StatusCode::PAYLOAD_TOO_LARGE,
            "Payload too large: received 2048 bytes, limit is 1024 bytes",
        ),
        (
            BodyParserError::MalformedMultipart("missing delimiter".to_string()),
            StatusCode::BAD_REQUEST,
            "Malformed multipart payload: missing delimiter",
        ),
        (
            BodyParserError::InvalidContentType("missing boundary".to_string()),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Invalid Content-Type header: missing boundary",
        ),
        (
            BodyParserError::UnsupportedEncoding("iso-8859-1".to_string()),
            StatusCode::UNSUPPORTED_MEDIA_TYPE,
            "Unsupported payload encoding: iso-8859-1",
        ),
        (
            BodyParserError::NestingDepthExceeded { max_depth: 10 },
            StatusCode::BAD_REQUEST,
            "Nesting depth limit of 10 levels exceeded",
        ),
        (
            BodyParserError::FieldCountExceeded { limit: 50 },
            StatusCode::BAD_REQUEST,
            "Maximum field count of 50 exceeded",
        ),
        (
            BodyParserError::IoError("permission denied".to_string()),
            StatusCode::INTERNAL_SERVER_ERROR,
            "I/O error during body parsing: permission denied",
        ),
        (
            BodyParserError::JsonError("unexpected token".to_string()),
            StatusCode::BAD_REQUEST,
            "Invalid JSON payload: unexpected token",
        ),
        (
            BodyParserError::FormError("invalid parameter syntax".to_string()),
            StatusCode::BAD_REQUEST,
            "Invalid form payload: invalid parameter syntax",
        ),
        (
            BodyParserError::Utf8Error("invalid utf-8 sequence".to_string()),
            StatusCode::BAD_REQUEST,
            "Invalid UTF-8 payload: invalid utf-8 sequence",
        ),
        (
            BodyParserError::FileAlreadyMoved,
            StatusCode::INTERNAL_SERVER_ERROR,
            "Uploaded file has already been moved to permanent storage",
        ),
    ];

    for (error, expected_status, expected_msg) in cases {
        assert_eq!(error.status_code(), expected_status);
        assert_eq!(format!("{error}"), expected_msg);
        assert!(error.source().is_none());
    }
}

#[test]
fn test_body_parser_error_from_conversions() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
    let parser_err = BodyParserError::from(io_err);
    assert!(
        matches!(parser_err, BodyParserError::IoError(ref msg) if msg.contains("file not found"))
    );

    let json_err: Result<serde_json::Value, _> = serde_json::from_str("{invalid");
    let parser_json_err = BodyParserError::from(json_err.unwrap_err());
    assert!(matches!(parser_json_err, BodyParserError::JsonError(_)));

    let qs_err = qs::QsError::InvalidSyntax("malformed bracket".to_string());
    let parser_qs_err = BodyParserError::from(qs_err);
    assert!(
        matches!(parser_qs_err, BodyParserError::FormError(ref msg) if msg.contains("malformed bracket"))
    );

    let mut utf8_bytes = vec![0x80];
    utf8_bytes.push(0x81);
    let parser_utf8_err = BodyParserError::from(std::str::from_utf8(&utf8_bytes).unwrap_err());
    assert!(matches!(parser_utf8_err, BodyParserError::Utf8Error(_)));
}

#[test]
fn test_body_parser_error_to_router_error() {
    let payload_err = BodyParserError::PayloadTooLarge {
        limit: 100,
        received: 200,
    };
    let router_err = RouterError::from(payload_err);
    assert!(matches!(
        router_err,
        RouterError::PayloadTooLarge {
            max_bytes: 100,
            received_bytes: 200
        }
    ));

    let json_err = BodyParserError::JsonError("bad json".to_string());
    assert!(matches!(
        RouterError::from(json_err),
        RouterError::DeserializationError(ref msg) if msg == "bad json"
    ));

    let form_err = BodyParserError::FormError("bad form".to_string());
    assert!(matches!(
        RouterError::from(form_err),
        RouterError::DeserializationError(ref msg) if msg == "bad form"
    ));

    let utf8_err = BodyParserError::Utf8Error("bad utf8".to_string());
    assert!(matches!(
        RouterError::from(utf8_err),
        RouterError::DeserializationError(ref msg) if msg == "bad utf8"
    ));

    let multipart_err = BodyParserError::MalformedMultipart("bad multipart".to_string());
    assert!(matches!(
        RouterError::from(multipart_err),
        RouterError::DeserializationError(ref msg) if msg == "bad multipart"
    ));

    let moved_err = BodyParserError::FileAlreadyMoved;
    assert!(matches!(
        RouterError::from(moved_err),
        RouterError::Internal(ref msg) if msg.contains("Uploaded file has already been moved")
    ));
}
