//! Unit tests for temporary file lifecycle and resilient persistence.

use std::{fs, io::Read};

use bodyparser::{BodyParserError, TempFile};

#[test]
fn test_temp_file_creation_and_write() {
    let mut temp = TempFile::new(None).unwrap();
    assert!(temp.path().exists());
    assert_eq!(temp.size(), 0);

    temp.write_chunk(b"Hello ").unwrap();
    temp.write_chunk(b"World!").unwrap();
    temp.flush().unwrap();

    assert_eq!(temp.size(), 12);

    let initial = temp.read_initial_bytes(5).unwrap();
    assert_eq!(&initial, b"Hello");
}

#[test]
fn test_temp_file_raii_cleanup_on_drop() {
    let path = {
        let temp = TempFile::new(None).unwrap();
        let path = temp.path().to_path_buf();
        assert!(path.exists());
        path
    };

    // Upon dropping without persist_to(), the file must be deleted from disk
    assert!(!path.exists());
}

#[test]
fn test_temp_file_persistence() {
    let dest_dir = tempfile::tempdir().unwrap();
    let dest_path = dest_dir.path().join("sub_dir").join("persisted.txt");

    let saved_path = {
        let mut temp = TempFile::new(None).unwrap();
        temp.write_chunk(b"Persistent Data").unwrap();
        temp.flush().unwrap();

        let target = temp.persist_to(&dest_path).unwrap();
        assert_eq!(target, dest_path);
        assert!(temp.is_persisted());

        // Attempting to persist again yields FileAlreadyMoved
        assert_eq!(
            temp.persist_to(&dest_path).unwrap_err(),
            BodyParserError::FileAlreadyMoved
        );

        target
    };

    // File must still exist after TempFile handle drops
    assert!(saved_path.exists());

    let mut contents = String::new();
    fs::File::open(&saved_path)
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "Persistent Data");

    let _ = fs::remove_file(saved_path);
}

#[test]
fn test_temp_file_write_after_moved() {
    let dest_dir = tempfile::tempdir().unwrap();
    let dest_path = dest_dir.path().join("moved.txt");

    let mut temp = TempFile::new(None).unwrap();
    temp.write_chunk(b"First").unwrap();
    temp.persist_to(&dest_path).unwrap();

    assert_eq!(
        temp.write_chunk(b"Second").unwrap_err(),
        BodyParserError::FileAlreadyMoved
    );
    assert_eq!(temp.flush().unwrap_err(), BodyParserError::FileAlreadyMoved);

    // read_initial_bytes directly from path when file handle is closed after persist
    let bytes_after_move = temp.read_initial_bytes(5).unwrap();
    assert_eq!(&bytes_after_move, b"First");
}

#[test]
fn test_temp_file_custom_directory_and_invalid_persist() {
    let custom_dir = tempfile::tempdir().unwrap();
    let mut temp = TempFile::new(Some(custom_dir.path())).unwrap();
    assert!(temp.path().starts_with(custom_dir.path()));
    assert!(!temp.is_persisted());

    temp.write_chunk(b"Custom directory payload").unwrap();
    temp.flush().unwrap();

    let initial = temp.read_initial_bytes(6).unwrap();
    assert_eq!(&initial, b"Custom");

    // Persisting to an existing directory itself as a file fails
    let fail_target = custom_dir.path().to_path_buf();
    let result = temp.persist_to(&fail_target);
    assert!(result.is_err());
    assert!(!temp.is_persisted());
}
