use std::fs::{create_dir_all, remove_file, write};

use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use dev_shared::check::CheckErrorExt;
use dev_shared::fs::{wait_until_modified, write_until_modified};
use dev_shared::task::CommonTask;
use dev_shared::test::{pie, temp_dir, TestPie};

#[rstest]
fn test_modified_stamp_on_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");

  // Modified stamper
  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  write(&path, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write_until_modified(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Modified recursive stamper; should work exactly the same as modified stamper when used on a file.
  // New task: execute
  let task = CommonTask::read_string_from_file(&path, FileStamper::ModifiedRecursive);
  write(&path, "hello world!").check();
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
  write_until_modified(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}

#[rstest]
fn test_modified_stamp_on_directory(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let dir_path = temp_dir.path().join("dir");
  create_dir_all(&dir_path).check();
  let file_path_1 = dir_path.join("test1.txt");

  // On some OSs, the modified time has low precision and therefore two subsequent writes can result in the same
  // modified time. Therefore, we will write to an unrelated file until its modified time changes, which signals that
  // the OS modified time has changed.
  let unrelated_dir_path = temp_dir.path().join("unrelated");
  create_dir_all(&unrelated_dir_path).check();
  let unrelated_file_path = unrelated_dir_path.join("unrelated.txt");

  // Modified stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::Modified);
  write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write_until_modified(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // File was added and this changes directory modified time: execution
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes directory modified time: execution
  write(&unrelated_file_path, "unrelated").check();
  wait_until_modified(&unrelated_file_path).check();
  remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Modified recursive stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::ModifiedRecursive);
  write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write_until_modified(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was added and this changes directory modified time: execution
  wait_until_modified(&unrelated_file_path).check();
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes directory modified time: execution
  wait_until_modified(&unrelated_file_path).check();
  remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}

#[rstest]
fn test_hash_stamp_on_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");

  // Hash stamper
  let task = CommonTask::read_string_from_file(&path, FileStamper::Hash);
  write(&path, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed: execute
  write(&path, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Hash recursive stamper; should work exactly the same as hash stamper when used on a file.
  let task = CommonTask::read_string_from_file(&path, FileStamper::HashRecursive);
  write(&path, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write(&path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed: execute
  write(&path, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}

#[rstest]
fn test_hash_stamp_on_directory(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let dir_path = temp_dir.path().join("dir");
  create_dir_all(&dir_path).check();
  let file_path_1 = dir_path.join("test1.txt");

  // Hash stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::Hash);
  write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed, but does not affect directory hash: no execution
  write(&file_path_1, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // File was added and this changes directory hash: execution
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes directory hash: execution
  remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });

  // Hash recursive stamper
  let task = CommonTask::list_directory(&dir_path, FileStamper::HashRecursive);
  write(&file_path_1, "hello world!").check();
  // New task: execute
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert!(session.dependency_check_errors().is_empty());

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
  write(&file_path_1, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
  // Stamp changed because file contents are changed: execute
  write(&file_path_1, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was added and this changes the hash: execution
  let file_path_2 = dir_path.join("test2.txt");
  write(&file_path_2, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
  // File was removed and this changes the hash: execution
  remove_file(&file_path_2).check();
  pie.run_in_session(|mut session| {
    session.require(&task).check();

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start_of(&task));
  });
}
