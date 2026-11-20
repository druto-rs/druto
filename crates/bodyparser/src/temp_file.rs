//! Temporary file lifecycle management with RAII cleanup and resilient persistence.

use std::{
    fs::{self, File},
    io::{self, Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

use crate::error::BodyParserError;

/// An on-disk temporary file handle with RAII deletion on drop unless explicitly persisted.
#[derive(Debug)]
pub struct TempFile {
    path: PathBuf,
    file: Option<File>,
    size: u64,
    persisted: bool,
}

impl TempFile {
    /// Creates a new temporary file in the specified directory, or the OS default temporary directory.
    pub fn new(directory: Option<&Path>) -> Result<Self, BodyParserError> {
        let named_temp = if let Some(dir) = directory {
            tempfile::Builder::new()
                .prefix("druto_upload_")
                .tempfile_in(dir)?
        } else {
            tempfile::Builder::new()
                .prefix("druto_upload_")
                .tempfile()?
        };

        // We take ownership of the path and the handle, managing RAII drop ourselves
        // to enable custom cross-device copy fallbacks during persistence.
        let (file, temp_path) = named_temp.into_parts();
        let path = temp_path.keep().map_err(|error| {
            BodyParserError::IoError(format!("Failed to retain temporary file path: {error}"))
        })?;

        Ok(Self {
            path,
            file: Some(file),
            size: 0,
            persisted: false,
        })
    }

    /// Returns the filesystem path to the temporary file.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns the number of bytes written to the temporary file.
    #[must_use]
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Appends a byte chunk to the temporary file.
    pub fn write_chunk(&mut self, chunk: &[u8]) -> Result<(), BodyParserError> {
        if let Some(file) = &mut self.file {
            file.write_all(chunk)?;
            self.size += chunk.len() as u64;
            Ok(())
        } else {
            Err(BodyParserError::FileAlreadyMoved)
        }
    }

    /// Flushes any pending buffered writes to disk.
    pub fn flush(&mut self) -> Result<(), BodyParserError> {
        if let Some(file) = &mut self.file {
            file.flush()?;
            Ok(())
        } else {
            Err(BodyParserError::FileAlreadyMoved)
        }
    }

    /// Reads up to `count` initial bytes from the beginning of the file for inspection without consuming the cursor.
    pub fn read_initial_bytes(&mut self, count: usize) -> Result<Vec<u8>, BodyParserError> {
        if let Some(file) = &mut self.file {
            file.flush()?;
            let current_pos = file.stream_position()?;
            file.seek(SeekFrom::Start(0))?;

            let mut buffer = vec![0u8; count];
            let bytes_read = file.read(&mut buffer)?;
            buffer.truncate(bytes_read);

            file.seek(SeekFrom::Start(current_pos))?;
            Ok(buffer)
        } else {
            // File handle was closed after move, read directly from path
            let mut file = File::open(&self.path)?;
            let mut buffer = vec![0u8; count];
            let bytes_read = file.read(&mut buffer)?;
            buffer.truncate(bytes_read);
            Ok(buffer)
        }
    }

    /// Persists the temporary file to a target destination path.
    ///
    /// Attempts an atomic [`fs::rename`] first. If moving across distinct filesystem mount points
    /// (triggering `EXDEV` / cross-device link error), falls back to copying the byte stream and
    /// unlinking the temporary source.
    pub fn persist_to(
        &mut self,
        destination: impl AsRef<Path>,
    ) -> Result<PathBuf, BodyParserError> {
        if self.persisted {
            return Err(BodyParserError::FileAlreadyMoved);
        }

        let target_path = destination.as_ref().to_path_buf();

        // 1. Ensure parent directories exist
        if let Some(parent) = target_path.parent()
            && !parent.as_os_str().is_empty()
        {
            fs::create_dir_all(parent)?;
        }

        // 2. Flush and close our open file handle so destination renames/copies succeed cleanly
        if let Some(mut file) = self.file.take() {
            file.flush()?;
            drop(file);
        }

        // 3. Attempt atomic filesystem rename
        match fs::rename(&self.path, &target_path) {
            Ok(()) => {
                self.persisted = true;
                self.path = target_path.clone();
                Ok(target_path)
            }
            Err(rename_error) => {
                // Cross-device move detected (EXDEV or Windows cross-volume error)
                if Self::is_cross_device_error(&rename_error) {
                    Self::copy_and_unlink(&self.path, &target_path)?;
                    self.persisted = true;
                    self.path = target_path.clone();
                    Ok(target_path)
                } else {
                    // Re-open handle if rename failed so Drop can still delete the tempfile
                    self.file = File::options().append(true).open(&self.path).ok();
                    Err(BodyParserError::from(rename_error))
                }
            }
        }
    }

    /// Copies source file to destination and unlinks source upon completion.
    fn copy_and_unlink(source: &Path, destination: &Path) -> Result<(), BodyParserError> {
        let mut reader = File::open(source)?;
        let mut writer = File::create(destination)?;
        io::copy(&mut reader, &mut writer)?;
        writer.sync_all()?;
        drop(reader);
        drop(writer);

        let _ = fs::remove_file(source);
        Ok(())
    }

    /// Returns `true` if the I/O error corresponds to a cross-device link failure.
    fn is_cross_device_error(error: &io::Error) -> bool {
        // ErrorKind::CrossesDevices is stable in modern Rust standard library
        if error.kind() == io::ErrorKind::CrossesDevices {
            return true;
        }
        // Raw OS error check for EXDEV (18 on Linux/POSIX)
        #[cfg(unix)]
        if error.raw_os_error() == Some(18) {
            return true;
        }
        false
    }

    /// Returns whether the file has been moved to permanent storage.
    #[must_use]
    pub fn is_persisted(&self) -> bool {
        self.persisted
    }
}

impl Drop for TempFile {
    fn drop(&mut self) {
        if !self.persisted && self.path.exists() {
            // Silently swallow remove errors during Drop to prevent unwinding panics
            let _ = fs::remove_file(&self.path);
        }
    }
}
