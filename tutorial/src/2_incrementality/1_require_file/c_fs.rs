use std::{fs, io};
use std::fs::{File, Metadata};
use std::path::Path;

/// Gets the metadata for given `path`, returning:
/// - `Ok(Some(metadata))` if a file or directory exists at given path,
/// - `Ok(None)` if no file or directory exists at given path,
/// - `Err(e)` if there was an error getting the metadata for given path.
pub fn metadata(path: impl AsRef<Path>) -> Result<Option<Metadata>, io::Error> {
  match fs::metadata(path) {
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(e),
    Ok(m) => Ok(Some(m))
  }
}

/// Attempt to open file at given `path`, returning:
/// - `Ok(Some(file))` if the file exists at given path,
/// - `Ok(None)` if no file exists at given path (but a directory could exist at given path),
/// - `Err(e)` if there was an error getting the metadata for given path, or if there was an error opening the file.
///
/// This function is necessary due to Windows returning an error when attempting to open a directory.
pub fn open_if_file(path: impl AsRef<Path>) -> Result<Option<File>, io::Error> {
  let file = match metadata(&path)? {
    Some(metadata) if metadata.is_file() => Some(File::open(&path)?),
    _ => None,
  };
  Ok(file)
}

