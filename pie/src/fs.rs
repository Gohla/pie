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
/// - `Ok(Some(file))` if a file exists at given path, 
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


#[cfg(test)]
mod test {
  use std::fs::remove_file;
  use std::io;

  use assert_matches::assert_matches;

  use dev_shared::{create_temp_dir, create_temp_file};

  use super::*;

  #[test]
  fn test_metadata_ok() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    let metadata = metadata(temp_file)?;
    assert_matches!(metadata, Some(metadata) => {
      assert!(metadata.is_file());  
    });
    Ok(())
  }

  #[test]
  fn test_metadata_none() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    remove_file(&temp_file)?;
    let metadata = metadata(&temp_file)?;
    assert!(metadata.is_none());
    Ok(())
  }

  #[test]
  fn test_open_if_file() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    let file = open_if_file(&temp_file)?;
    assert!(file.is_some());
    Ok(())
  }

  #[test]
  fn test_open_if_file_non_existent() -> Result<(), io::Error> {
    let temp_file = create_temp_file()?;
    remove_file(&temp_file)?;
    let file = open_if_file(&temp_file)?;
    assert!(file.is_none());
    Ok(())
  }

  #[test]
  fn test_open_if_file_on_directory() -> Result<(), io::Error> {
    let temp_dir = create_temp_dir()?;
    let file = open_if_file(temp_dir)?;
    assert!(file.is_none());
    Ok(())
  }
}
