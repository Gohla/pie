use tempfile::{NamedTempFile, TempDir};

pub fn create_temp_file() -> NamedTempFile {
  NamedTempFile::new().expect("failed to create temporary file")
}

pub fn create_temp_dir() -> TempDir {
  TempDir::new().expect("failed to create temporary directory")
}
