use std::io;

use tempfile::{NamedTempFile, TempDir};

/// Creates a new temporary file that gets cleaned up when dropped.
pub fn create_temp_file() -> Result<NamedTempFile, io::Error> { NamedTempFile::new() }

/// Creates a new temporary directory that gets cleaned up when dropped.
pub fn create_temp_dir() -> Result<TempDir, io::Error> { TempDir::new() }
