use std::error::Error;

use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::{FileStamp, FileStamper};
use dev_shared::task::CommonTask;
use dev_shared::test::{pie, temp_dir, TestPie, TestPieExt};

#[rstest]
fn test_dependencies_to_non_existent_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  pie.require_then_assert(&CommonTask::read_string_from_file(&path, FileStamper::Modified), |tracker|
    assert!(tracker.contains_one_require_file_start_of_with(&path, |s| s == &FileStamp::Modified(None)))
  )?;
  pie.require_then_assert(&CommonTask::read_string_from_file(&path, FileStamper::Hash), |tracker|
    assert!(tracker.contains_one_require_file_start_of_with(&path, |s| s == &FileStamp::Hash(None)))
  )?;
  Ok(())
}
