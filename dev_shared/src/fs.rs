use std::fs;
use std::path::Path;
use std::time::SystemTime;

use tempfile::{NamedTempFile, TempDir};

pub fn create_temp_file() -> NamedTempFile {
  NamedTempFile::new().expect("failed to create temporary file")
}

pub fn create_temp_dir() -> TempDir {
  TempDir::new().expect("failed to create temporary directory")
}

pub fn write_until_modified(file_path: impl AsRef<Path>, contents: impl AsRef<[u8]>) {
  let file_path = file_path.as_ref();
  let contents = contents.as_ref();
  fn get_modified(path: impl AsRef<Path>) -> SystemTime {
    fs::metadata(path)
      .expect("failed to get metadata")
      .modified()
      .expect("failed to get modified time")
  }
  let modified = get_modified(file_path);
  // Keep writing to file until modified time changes, because some modified time implementations have low precision 
  // and do not change after writing.
  loop {
    fs::write(file_path, contents).expect("failed to write to file");
    if modified != get_modified(file_path) { break; }
  }
}
