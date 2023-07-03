use std::fs::{metadata, write};
use std::path::Path;
use std::time::SystemTime;

use tempfile::{NamedTempFile, TempDir};

/// Creates a new temporary file that gets cleaned up when dropped.
#[inline]
pub fn create_temp_file() -> NamedTempFile {
  NamedTempFile::new().expect("failed to create temporary file")
}

/// Creates a new temporary directory that gets cleaned up when dropped.
#[inline]
pub fn create_temp_dir() -> TempDir {
  TempDir::new().expect("failed to create temporary directory")
}

/// Keeps writing `contents` to file at `path` until it's last modified time changes, then returns the modified time.
///
/// # Errors
///
/// Returns an error when any file operation fails.
pub fn write_until_modified(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) -> Result<SystemTime, std::io::Error> {
  let path = path.as_ref();
  let contents = contents.as_ref();
  fn get_modified(path: impl AsRef<Path>) -> Result<SystemTime, std::io::Error> {
    let modified = match metadata(path) {
      Err(e) if e.kind() == std::io::ErrorKind::NotFound => SystemTime::UNIX_EPOCH,
      Err(e) => Err(e)?,
      Ok(m) => m.modified()?
    };
    Ok(modified)
  }
  let modified = get_modified(path)?;
  // Keep writing to file until its modified time changes, because some modified time implementations have low precision 
  // and do not change after writing in quick succession.
  loop {
    write(path, contents)?;
    if modified != get_modified(path)? { break; }
  }
  Ok(modified)
}

/// First writes to a temporary file, and then keeps writing to it until it's last modified time changes, ensuring that
/// the OS last modified time has changed.
pub fn wait_until_modified_time_changes() -> Result<SystemTime, std::io::Error> {
  let file = create_temp_file();
  write(&file, "123")?;
  write_until_modified(&file, "123")
}
