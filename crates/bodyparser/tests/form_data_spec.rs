use std::collections::BTreeMap;

use bodyparser::{FormFields, TempFile, UploadedFile, UploadedFiles};
use qs::QsValue;

fn make_test_file(field: &str, filename: &str) -> UploadedFile {
    let mut temp = TempFile::new(None).expect("failed to create tempfile");
    temp.write_chunk(b"hello content")
        .expect("failed to write chunk");
    UploadedFile::new(
        field,
        filename,
        Some("text/plain".to_string()),
        "text/plain",
        temp,
    )
}

#[test]
fn test_uploaded_files_container() {
    let mut files = UploadedFiles::default();
    assert!(files.is_empty());
    assert_eq!(files.len(), 0);
    assert_eq!(files.all().len(), 0);

    let file_one = make_test_file("avatar", "avatar1.png");
    let file_two = make_test_file("documents[]", "doc1.pdf");
    let file_three = make_test_file("documents[]", "doc2.pdf");

    files.push(file_one);
    files.push(file_two);
    files.push(file_three);

    assert!(!files.is_empty());
    assert_eq!(files.len(), 3);

    // Single lookup
    let avatar = files.file("avatar").expect("avatar file missing");
    assert_eq!(avatar.client_filename(), "avatar1.png");
    assert!(files.file("nonexistent").is_none());

    // Single mutable lookup
    let avatar_mut = files.file_mut("avatar").expect("avatar file mut missing");
    assert_eq!(avatar_mut.client_filename(), "avatar1.png");

    // Multiple lookup by name
    let docs = files.files_by_name("documents[]");
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].client_filename(), "doc1.pdf");
    assert_eq!(docs[1].client_filename(), "doc2.pdf");

    // All and all_mut
    assert_eq!(files.all().len(), 3);
    assert_eq!(files.all_mut().len(), 3);

    // into_inner and new
    let extracted = files.into_inner();
    assert_eq!(extracted.len(), 3);

    let rewrapped = UploadedFiles::new(extracted);
    assert_eq!(rewrapped.len(), 3);
}

#[test]
fn test_form_fields_container() {
    let default_fields = FormFields::default();
    assert!(default_fields.get("name").is_none());
    assert_eq!(default_fields.raw().count(), 0);
    assert!(matches!(default_fields.tree(), QsValue::Object(map) if map.is_empty()));

    let raw_pairs = vec![
        ("user".to_string(), "alice".to_string()),
        ("role".to_string(), "member".to_string()),
        ("role".to_string(), "admin".to_string()),
    ];

    let mut map = BTreeMap::new();
    map.insert("user".to_string(), QsValue::Scalar("alice".to_string()));
    map.insert("role".to_string(), QsValue::Scalar("admin".to_string()));
    let tree = QsValue::Object(map);

    let form = FormFields::new(raw_pairs, tree);

    // get returns the last value for duplicate keys
    assert_eq!(form.get("user"), Some("alice"));
    assert_eq!(form.get("role"), Some("admin"));
    assert_eq!(form.get("unknown"), None);

    // raw iterator
    let collected_raw: Vec<(&str, &str)> = form.raw().collect();
    assert_eq!(collected_raw.len(), 3);
    assert_eq!(collected_raw[0], ("user", "alice"));
    assert_eq!(collected_raw[1], ("role", "member"));
    assert_eq!(collected_raw[2], ("role", "admin"));

    // tree
    assert!(matches!(form.tree(), QsValue::Object(m) if m.len() == 2));
}
