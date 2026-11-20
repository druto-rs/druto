//! Unit and property tests for payload quotas and limit guards.

use bodyparser::{
    BodyParserError, DEFAULT_MAX_BODY_SIZE, DEFAULT_MAX_DEPTH, DEFAULT_MAX_FIELDS_COUNT,
    DEFAULT_MAX_FIELDS_SIZE, DEFAULT_MAX_FILE_SIZE, PayloadLimits,
};

#[test]
fn test_default_limits() {
    let limits = PayloadLimits::default();
    assert_eq!(limits.max_body_size(), DEFAULT_MAX_BODY_SIZE);
    assert_eq!(limits.max_file_size(), DEFAULT_MAX_FILE_SIZE);
    assert_eq!(limits.max_fields_size(), DEFAULT_MAX_FIELDS_SIZE);
    assert_eq!(limits.max_fields_count(), DEFAULT_MAX_FIELDS_COUNT);
    assert_eq!(limits.max_depth(), DEFAULT_MAX_DEPTH);
}

#[test]
fn test_custom_limits_builder() {
    let limits = PayloadLimits::builder()
        .max_body_size(50 * 1024 * 1024)
        .max_file_size(25 * 1024 * 1024)
        .max_fields_size(2 * 1024 * 1024)
        .max_fields_count(500)
        .max_depth(50)
        .build();

    assert_eq!(limits.max_body_size(), 50 * 1024 * 1024);
    assert_eq!(limits.max_file_size(), 25 * 1024 * 1024);
    assert_eq!(limits.max_fields_size(), 2 * 1024 * 1024);
    assert_eq!(limits.max_fields_count(), 500);
    assert_eq!(limits.max_depth(), 50);
}

#[test]
fn test_check_body_size() {
    let limits = PayloadLimits::builder().max_body_size(100).build();

    assert!(limits.check_body_size(50).is_ok());
    assert!(limits.check_body_size(100).is_ok());

    let err = limits.check_body_size(101).unwrap_err();
    assert_eq!(
        err,
        BodyParserError::PayloadTooLarge {
            limit: 100,
            received: 101,
        }
    );
}

#[test]
fn test_check_file_size() {
    let limits = PayloadLimits::builder().max_file_size(200).build();

    assert!(limits.check_file_size(199).is_ok());
    assert!(limits.check_file_size(200).is_ok());

    let err = limits.check_file_size(201).unwrap_err();
    assert_eq!(
        err,
        BodyParserError::PayloadTooLarge {
            limit: 200,
            received: 201,
        }
    );
}

#[test]
fn test_check_fields_size() {
    let limits = PayloadLimits::builder().max_fields_size(300).build();

    assert!(limits.check_fields_size(300).is_ok());
    let err = limits.check_fields_size(301).unwrap_err();
    assert_eq!(
        err,
        BodyParserError::PayloadTooLarge {
            limit: 300,
            received: 301,
        }
    );
}

#[test]
fn test_check_fields_count() {
    let limits = PayloadLimits::builder().max_fields_count(10).build();

    assert!(limits.check_fields_count(10).is_ok());
    let err = limits.check_fields_count(11).unwrap_err();
    assert_eq!(err, BodyParserError::FieldCountExceeded { limit: 10 });
}

#[test]
fn test_check_depth() {
    let limits = PayloadLimits::builder().max_depth(5).build();

    assert!(limits.check_depth(5).is_ok());
    let err = limits.check_depth(6).unwrap_err();
    assert_eq!(err, BodyParserError::NestingDepthExceeded { max_depth: 5 });
}
