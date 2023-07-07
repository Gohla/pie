use std::error::Error;

use assert_matches::assert_matches;
use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::{FileStamp, FileStamper};
use dev_shared_external::task::*;
use dev_shared_external::test::{pie, temp_dir, TestPie, TestPieExt};

#[rstest]
fn test_dependencies_to_non_existent_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  pie.require_then_assert(&ReadStringFromFile::new(&path, FileStamper::Modified), |tracker|
    assert_matches!(tracker.find_require_file(&path), Some(FileStamp::Modified(None))),
  )?;
  pie.require_then_assert(&ReadStringFromFile::new(&path, FileStamper::Hash), |tracker|
    assert_matches!(tracker.find_require_file(&path), Some(FileStamp::Hash(None))),
  )?;
  Ok(())
}
