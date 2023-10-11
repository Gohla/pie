use std::fs::{metadata, write};
use std::io;
use std::path::Path;
use std::time::SystemTime;

use tempfile::{NamedTempFile, TempDir};

/// Creates a new temporary file that gets cleaned up when dropped.
pub fn create_temp_file() -> Result<NamedTempFile, io::Error> { NamedTempFile::new() }

/// Creates a new temporary directory that gets cleaned up when dropped.
pub fn create_temp_dir() -> Result<TempDir, io::Error> { TempDir::new() }

/// Keeps writing `contents` to file at `path` until its last modified time changes, then returns the modified time.
/// This is required because some OSs have imprecise modified timers, where the file modified time does not change when
/// writing in quick succession.
///
/// # Errors
///
/// Returns an error when any file operation fails.
pub fn write_until_modified(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<SystemTime, io::Error> {
  let path = path.as_ref();
  let contents = contents.as_ref();
  fn get_modified(path: impl AsRef<Path>) -> Result<SystemTime, io::Error> {
    let modified = match metadata(path) {
      Err(e) if e.kind() == io::ErrorKind::NotFound => SystemTime::UNIX_EPOCH,
      Err(e) => Err(e)?,
      Ok(m) => m.modified()?
    };
    Ok(modified)
  }
  let modified = get_modified(path)?;
  loop {
    write(path, contents)?;
    if modified != get_modified(path)? { break; }
  }
  Ok(modified)
}
