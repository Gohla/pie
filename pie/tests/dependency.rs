use rstest::{fixture, rstest};
use tempfile::TempDir;

use dev_shared::task::CommonTask;
use dev_shared::TestPie;
use ::pie::stamp::{FileStamp, FileStamper};

#[fixture]
fn pie() -> TestPie<CommonTask> { dev_shared::create_test_pie() }

#[fixture]
fn temp_dir() -> TempDir { dev_shared::fs::create_temp_dir() }


#[rstest]
fn test_dependencies_to_non_existent_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("in.txt");

  pie.run_in_session(|mut session| {
    session.require(&CommonTask::read_string_from_file(&path, FileStamper::Modified));
    assert_eq!(session.dependency_check_errors().len(), 0);
    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_require_file_start_of_with(&path, |s| s == &FileStamp::Modified(None)));
  });

  pie.run_in_session(|mut session| {
    session.require(&CommonTask::read_string_from_file(&path, FileStamper::Hash));
    assert_eq!(session.dependency_check_errors().len(), 0);
    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_require_file_start_of_with(&path, |s| s == &FileStamp::Hash(None)));
  });
}
