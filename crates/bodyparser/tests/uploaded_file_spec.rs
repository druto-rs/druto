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
