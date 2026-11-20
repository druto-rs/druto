//! Unit tests for UploadedFile metadata, validation rules, and movements.

use bodyparser::{FileError, FileValidator, TempFile, UploadedFile};

#[test]
fn test_uploaded_file_metadata() {
    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"fake image data").unwrap();
    temp.flush().unwrap();

    let file = UploadedFile::new(
        "avatar",
        "profile_picture.png",
        Some("image/png".to_string()),
        "image/png",
        temp,
    );

    assert_eq!(file.field_name(), "avatar");
    assert_eq!(file.client_filename(), "profile_picture.png");
    assert_eq!(file.client_content_type(), Some("image/png"));
    assert_eq!(file.detected_mime_type(), "image/png");
    assert_eq!(file.extension(), Some("png"));
    assert_eq!(file.size(), 15);
    assert!(file.is_valid());
    assert!(!file.has_errors());
    assert_eq!(file.errors().len(), 0);
    assert!(!file.is_moved());
}

#[test]
fn test_sanitize_filename() {
    assert_eq!(
        UploadedFile::sanitize_filename("../../../etc/passwd"),
        "passwd"
    );
    assert_eq!(
        UploadedFile::sanitize_filename("C:\\Windows\\System32\\cmd.exe"),
        "cmd.exe"
    );
    assert_eq!(
        UploadedFile::sanitize_filename("valid_photo.jpg"),
        "valid_photo.jpg"
    );
    assert_eq!(UploadedFile::sanitize_filename("..."), "uploaded_file");
    assert_eq!(
        UploadedFile::sanitize_filename("foo\0bar.txt"),
        "foobar.txt"
    );
}

#[test]
fn test_file_validator_extension() {
    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"data").unwrap();
    temp.flush().unwrap();

    let mut file = UploadedFile::new(
        "document",
        "contract.exe",
        None,
        "application/octet-stream",
        temp,
    );

    let validator = FileValidator::new().extensions(["pdf", "docx"]);
    file.validate(&validator);

    assert!(!file.is_valid());
    assert_eq!(
        file.errors(),
        &[FileError::InvalidExtension {
            extension: "exe".to_string(),
            allowed: vec!["pdf".to_string(), "docx".to_string()],
        }]
    );
}

#[test]
fn test_file_validator_size() {
    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"1234567890").unwrap(); // 10 bytes
    temp.flush().unwrap();

    let mut file = UploadedFile::new("doc", "file.txt", None, "text/plain", temp);

    let validator = FileValidator::new().max_size(5).min_size(20);
    file.validate(&validator);

    assert!(!file.is_valid());
    assert_eq!(file.errors().len(), 2);
    assert_eq!(
        file.errors()[0],
        FileError::SizeLimitExceeded {
            limit: 5,
            actual: 10
        }
    );
    assert_eq!(
        file.errors()[1],
        FileError::SizeBelowMinimum {
            minimum: 20,
            actual: 10
        }
    );
}

#[test]
fn test_file_validator_mime_wildcard() {
    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"data").unwrap();
    temp.flush().unwrap();

    let mut file = UploadedFile::new("avatar", "photo.png", None, "image/png", temp);

    let validator = FileValidator::new().mime("image/*");
    file.validate(&validator);
    assert!(file.is_valid());
}

#[test]
fn test_uploaded_file_move_to() {
    let dest_dir = tempfile::tempdir().unwrap();
    let dest_file = dest_dir.path().join("final").join("stored.png");

    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"file contents").unwrap();
    temp.flush().unwrap();

    let mut file = UploadedFile::new("avatar", "photo.png", None, "image/png", temp);

    let final_path = file.move_to(&dest_file).unwrap();
    assert_eq!(final_path, dest_file);
    assert!(file.is_moved());
    assert!(file.path().exists());

    // Attempting to move again yields error
    assert_eq!(
        file.move_to(&dest_file).unwrap_err(),
        bodyparser::BodyParserError::FileAlreadyMoved
    );
}

#[test]
fn test_file_error_display_and_source() {
    let cases = vec![
        (
            FileError::SizeLimitExceeded {
                limit: 100,
                actual: 200,
            },
            "File size of 200 bytes exceeds maximum limit of 100 bytes",
        ),
        (
            FileError::SizeBelowMinimum {
                minimum: 50,
                actual: 10,
            },
            "File size of 10 bytes is below required minimum of 50 bytes",
        ),
        (
            FileError::InvalidExtension {
                extension: "exe".to_string(),
                allowed: vec!["jpg".to_string(), "png".to_string()],
            },
            "File extension 'exe' is not allowed. Allowed extensions: jpg, png",
        ),
        (
            FileError::InvalidMimeType {
                mime: "text/plain".to_string(),
                allowed: vec!["image/jpeg".to_string(), "image/png".to_string()],
            },
            "MIME type 'text/plain' is not allowed. Allowed types: image/jpeg, image/png",
        ),
        (
            FileError::MimeMismatch {
                declared: "image/png".to_string(),
                detected: "application/x-executable".to_string(),
            },
            "Detected content signature 'application/x-executable' does not match declared type 'image/png'",
        ),
    ];

    for (error, expected) in cases {
        assert_eq!(format!("{error}"), expected);
        use std::error::Error;
        assert!(error.source().is_none());
    }
}

#[test]
fn test_file_validator_extension_clean_and_extensionless() {
    let mut temp1 = TempFile::new(None).unwrap();
    temp1.write_chunk(b"data").unwrap();
    let mut file_with_dot = UploadedFile::new("f", "doc.pdf", None, "application/pdf", temp1);

    let validator = FileValidator::new().extension(".pdf");
    file_with_dot.validate(&validator);
    assert!(file_with_dot.is_valid());

    let mut temp2 = TempFile::new(None).unwrap();
    temp2.write_chunk(b"data").unwrap();
    let mut file_no_ext = UploadedFile::new("f", "LICENSE", None, "text/plain", temp2);

    file_no_ext.validate(&validator);
    assert!(!file_no_ext.is_valid());
    assert!(file_no_ext.has_errors());
    assert_eq!(
        file_no_ext.errors()[0],
        FileError::InvalidExtension {
            extension: String::new(),
            allowed: vec!["pdf".to_string()],
        }
    );
}

#[test]
fn test_file_validator_mime_exact_and_mismatches() {
    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"binary payload").unwrap();
    let mut file = UploadedFile::new("upload", "archive.tar", None, "application/x-tar", temp);

    let validator = FileValidator::new().mimes(["application/zip", "application/gzip"]);
    file.validate(&validator);

    assert!(!file.is_valid());
    assert_eq!(
        file.errors(),
        &[FileError::InvalidMimeType {
            mime: "application/x-tar".to_string(),
            allowed: vec![
                "application/zip".to_string(),
                "application/gzip".to_string()
            ],
        }]
    );
}
