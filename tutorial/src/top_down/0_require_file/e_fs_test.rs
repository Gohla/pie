#[cfg(test)]
mod test {
  use std::fs::remove_file;

  use tempfile::{NamedTempFile, TempDir, TempPath};

  use super::*;

  #[test]
  fn test_metadata_ok() {
    let path = create_temp_path();
    let metadata = metadata(path);
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap();
    assert!(metadata.is_some());
    let metadata = metadata.unwrap();
    assert!(metadata.is_file());
  }

  #[test]
  fn test_metadata_none() {
    let path = create_temp_path();
    remove_file(&path).expect("failed to delete temporary file");
    let metadata = metadata(&path);
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap();
    assert!(metadata.is_none());
  }

  #[test]
  fn test_open_if_file() {
    let path = create_temp_path();
    let file = open_if_file(&path);
    assert!(file.is_ok());
    let file = file.unwrap();
    assert!(file.is_some());
  }

  #[test]
  fn test_open_if_file_non_existent() {
    let path = create_temp_path();
    remove_file(&path).expect("failed to delete temporary file");
    let file = open_if_file(&path);
    assert!(file.is_ok());
    let file = file.unwrap();
    assert!(file.is_none());
  }

  #[test]
  fn test_open_if_file_on_directory() {
    let dir = TempDir::new().expect("failed to create temporary directory");
    let file = open_if_file(dir.path());
    assert!(file.is_ok());
    let file = file.unwrap();
    assert!(file.is_none());
  }

  fn create_temp_path() -> TempPath {
    NamedTempFile::new().expect("failed to create temporary file").into_temp_path()
  }
}
