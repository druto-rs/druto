use std::path::Path;

use bodyparser::{BodyParserConfig, PayloadLimits};
use poth::Method;

#[test]
fn test_body_parser_config_defaults() {
    let config = BodyParserConfig::default();
    assert_eq!(config.allowed_methods().len(), 5);
    assert!(config.is_method_allowed(&Method::Post));
    assert!(config.is_method_allowed(&Method::Put));
    assert!(config.is_method_allowed(&Method::Patch));
    assert!(config.is_method_allowed(&Method::Delete));
    assert!(config.is_method_allowed(&Method::Query));
    assert!(!config.is_method_allowed(&Method::Get));
    assert!(!config.is_method_allowed(&Method::Head));
    assert!(config.auto_process_files());
    assert!(config.process_manually_routes().is_empty());
    assert_eq!(config.temp_dir(), None);
    assert_eq!(config.limits().max_body_size(), 20 * 1024 * 1024);
}

#[test]
fn test_body_parser_config_builder_and_manual_processing() {
    let custom_limits = PayloadLimits::builder().max_body_size(1024).build();
    let config = BodyParserConfig::builder()
        .allowed_methods(vec![Method::Post, Method::Put])
        .limits(custom_limits)
        .auto_process_files(true)
        .process_manually("/upload/raw")
        .process_manually("/stream/*")
        .temp_dir("/tmp/custom_uploads")
        .build();

    assert_eq!(config.allowed_methods(), &[Method::Post, Method::Put]);
    assert!(config.is_method_allowed(&Method::Post));
    assert!(!config.is_method_allowed(&Method::Delete));
    assert_eq!(config.limits().max_body_size(), 1024);
    assert_eq!(config.process_manually_routes().len(), 2);
    assert_eq!(config.temp_dir(), Some(Path::new("/tmp/custom_uploads")));

    // should_process_manually: exact match
    assert!(config.should_process_manually("/upload/raw"));
    // should_process_manually: wildcard prefix match
    assert!(config.should_process_manually("/stream/videos"));
    assert!(config.should_process_manually("/stream/audio/123"));
    // should_process_manually: non-matching route
    assert!(!config.should_process_manually("/other/path"));

    // When auto_process_files is false, all routes should process manually
    let disabled_auto_config = BodyParserConfig::builder()
        .auto_process_files(false)
        .build();
    assert!(!disabled_auto_config.auto_process_files());
    assert!(disabled_auto_config.should_process_manually("/any/route"));
}
