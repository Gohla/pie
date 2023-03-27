use std::{fs, io};
use std::fs::{File, Metadata};
use std::path::Path;

pub fn metadata(path: impl AsRef<Path>) -> Result<Option<Metadata>, io::Error> {
  match fs::metadata(path) {
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(e),
    Ok(m) => Ok(Some(m))
  }
}

/// Open file at given path if it exists and is a file. This is necessary because on Windows, opening a directory
/// returns an error.
pub fn open_if_file(path: impl AsRef<Path>) -> Result<Option<File>, io::Error> {
  let file = match metadata(&path)? {
    Some(metadata) if metadata.is_file() => Some(File::open(&path)?),
    _ => None,
  };
  Ok(file)
}
