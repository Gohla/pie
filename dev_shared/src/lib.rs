use tempfile::TempDir;

pub mod check;
pub mod task;
pub mod test;
pub mod bench;

pub fn create_temp_dir() -> TempDir {
  tempfile::tempdir().expect("failed to create temporary directory")
}
