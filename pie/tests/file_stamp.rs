use std::fs;

use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use dev_shared::check::CheckErrorExt;
use dev_shared::task::CommonTask;
use dev_shared::test::Pie;

#[fixture]
fn pie() -> Pie<CommonTask> { dev_shared::test::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { dev_shared::create_temp_dir() }


#[rstest]
fn test_modified_stamp_on_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");

  // Modified stamper
  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  fs::write(&path, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed even though file contents is the same: execute
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Modified recursive stamper; should work exactly the same as modified stamper when used on a file.
  // New task: execute
  let task = CommonTask::read_string_from_file(&path, FileStamper::ModifiedRecursive);
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed even though file contents is the same: execute
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}

#[cfg(not(windows))] // These tests are flaky on Windows, due to modification dates not updating directly?
#[rstest]
fn test_modified_stamp_on_directory(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let dir_path = temp_dir.path().join("dir");
  fs::create_dir_all(&dir_path).check();
  let file_path_1 = dir_path.join("test1.txt");

  // Modified stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::Modified);
  fs::write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // File was changed but this does not affect directory modified time: no execution
  fs::write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // File was added and this changes directory modified time: execution
  let file_path_2 = dir_path.join("test2.txt");
  fs::write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes directory modified time: execution
  fs::remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Modified recursive stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::ModifiedRecursive);
  fs::write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // File was changed and this affects the latest modified date: execute
  fs::write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was added and this changes directory modified time: execution
  let file_path_2 = dir_path.join("test2.txt");
  fs::write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes directory modified time: execution
  fs::remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}

#[rstest]
fn test_hash_stamp_on_file(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");

  // Hash stamper
  let task = CommonTask::read_string_from_file(&path, FileStamper::Hash);
  fs::write(&path, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp unchanged because file contents are unchanged: no execution
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed: execute
  fs::write(&path, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Hash recursive stamper; should work exactly the same as hash stamper when used on a file.
  let task = CommonTask::read_string_from_file(&path, FileStamper::HashRecursive);
  fs::write(&path, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp unchanged because file contents are unchanged: no execution
  fs::write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed: execute
  fs::write(&path, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}

#[rstest]
fn test_hash_stamp_on_directory(mut pie: Pie<CommonTask>, temp_dir: TempDir) {
  let dir_path = temp_dir.path().join("dir");
  fs::create_dir_all(&dir_path).check();
  let file_path_1 = dir_path.join("test1.txt");

  // Hash stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::Hash);
  fs::write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp unchanged because file contents are unchanged: no execution
  fs::write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed, but does not affect directory hash: no execution
  fs::write(&file_path_1, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // File was added and this changes directory hash: execution
  let file_path_2 = dir_path.join("test2.txt");
  fs::write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes directory hash: execution
  fs::remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Hash recursive stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::HashRecursive);
  fs::write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // Stamp unchanged: no execution
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp unchanged because file contents are unchanged: no execution
  fs::write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed: execute
  fs::write(&file_path_1, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was added and this changes the hash: execution
  let file_path_2 = dir_path.join("test2.txt");
  fs::write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes the hash: execution
  fs::remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}
