use std::fs;

use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::dependency::FileStamper;

use crate::common::{CheckErrorExt, CommonTask, Pie};

mod common;

#[fixture]
fn pie() -> Pie<CommonTask> { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_modified_stamp_on_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "hello world!").check();

  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);

  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
    tracker.clear();
  });

  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Stamp changed even though file contents is the same: execute
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
    tracker.clear();
  });
}

#[rstest]
fn test_modified_stamp_on_directory(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let dir_path = temp_dir.path().join("dir");
  fs::create_dir_all(&dir_path).check();
  let file_path_1 = dir_path.join("test1.txt");
  fs::write(&file_path_1, "hello world!").check();

  let task = CommonTask::list_directory(&dir_path, FileStamper::Modified);

  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
    tracker.clear();
  });

  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // File was changed but this does not affect directory modified time: no execution
  fs::write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // File was added and this changes directory modified time: execution
  let file_path_2 = dir_path.join("test2.txt");
  fs::write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
    tracker.clear();
  });
}

#[rstest]
fn test_hash_stamp_on_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "hello world!").check();

  let task = CommonTask::read_string_from_file(&path, FileStamper::Hash);

  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
    tracker.clear();
  });

  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Stamp unchanged because has is unchanged: no execution
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });
}
