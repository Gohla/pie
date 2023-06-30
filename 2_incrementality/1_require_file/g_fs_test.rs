#[cfg(test)]
mod test {
  use std::fs::remove_file;

  use dev_shared::{create_temp_dir, create_temp_file};

  use super::*;

  #[test]
  fn test_metadata_ok() {
    let temp_file = create_temp_file();
    let metadata = metadata(temp_file);
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap();
    assert!(metadata.is_some());
    let metadata = metadata.unwrap();
    assert!(metadata.is_file());
  }

  #[test]
  fn test_metadata_none() {
    let temp_file = create_temp_file();
    remove_file(&temp_file).expect("failed to delete temporary file");
    let metadata = metadata(&temp_file);
    assert!(metadata.is_ok());
    let metadata = metadata.unwrap();
    assert!(metadata.is_none());
  }

  #[test]
  fn test_open_if_file() {
    let temp_file = create_temp_file();
    let file = open_if_file(&temp_file);
    assert!(file.is_ok());
    let file = file.unwrap();
    assert!(file.is_some());
  }

  #[test]
  fn test_open_if_file_non_existent() {
    let temp_file = create_temp_file();
    remove_file(&temp_file).expect("failed to delete temporary file");
    let file = open_if_file(&temp_file);
    assert!(file.is_ok());
    let file = file.unwrap();
    assert!(file.is_none());
  }

  #[test]
  fn test_open_if_file_on_directory() {
    let temp_dir = create_temp_dir();
    let file = open_if_file(temp_dir);
    assert!(file.is_ok());
    let file = file.unwrap();
    assert!(file.is_none());
  }
}
