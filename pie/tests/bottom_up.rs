use std::fs::{read_to_string, write};

use assert_matches::assert_matches;
use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use dev_shared::check::CheckErrorExt;
use dev_shared::fs::write_until_modified;
use dev_shared::task::{CommonOutput, CommonTask};
use dev_shared::TestPie;

#[fixture]
fn pie() -> TestPie<CommonTask> { dev_shared::create_test_pie() }

#[fixture]
fn temp_dir() -> TempDir { dev_shared::fs::create_temp_dir() }


#[rstest]
fn test_nothing_affected(mut pie: TestPie<CommonTask>) {
  pie.run_in_session(|mut session| {
    session.update_affected_by(&[]);
    assert!(session.dependency_check_errors().is_empty());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
  });
}

#[rstest]
fn test_directly_affected_task(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  write(&path, "HELLO WORLD!").check();

  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::read_string_from_file_ok("HELLO WORLD!"));
    assert!(session.dependency_check_errors().is_empty());
  });

  // Change the file that ReadStringFromFile requires, directly affecting it.
  write_until_modified(&path, "hello world!").check();

  pie.run_in_session(|mut session| {
    session.update_affected_by(&[path]);
    assert!(session.dependency_check_errors().is_empty());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_end_of_with(&task, &CommonOutput::read_string_from_file_ok("hello world!")));
  });
}

#[rstest]
fn test_indirectly_affected_tasks(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!").check();

  let read_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  let task = CommonTask::to_lower_case(read_task.clone());

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), CommonOutput::to_lower_case_ok("hello world!"));
    assert!(session.dependency_check_errors().is_empty());
  });

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting ToLowerCase, 
  // indirectly affecting CombineA.
  write_until_modified(&path, "HELLO WORLD!!").check();

  pie.run_in_session(|mut session| {
    session.update_affected_by(&[path.clone()]);
    assert!(session.dependency_check_errors().is_empty());

    let tracker = &mut session.tracker_mut().0;
    let read_task_end = tracker.get_index_of_execute_end_of_with(&read_task, &CommonOutput::read_string_from_file_ok("HELLO WORLD!!"));
    assert_matches!(read_task_end, Some(_));
    let task_end = tracker.get_index_of_execute_end_of_with(&task, &CommonOutput::to_lower_case_ok("hello world!!"));
    assert_matches!(task_end, Some(_));
    assert!(task_end > read_task_end);
  });
}

#[rstest]
fn test_indirectly_affected_tasks_early_cutoff(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "HELLO WORLD!").check();
  let write_path = temp_dir.path().join("out.txt");

  let read_task = CommonTask::read_string_from_file(&read_path, FileStamper::Modified);
  let to_lowercase_task = CommonTask::to_lower_case(read_task.clone());
  let write_task = CommonTask::write_string_to_file(to_lowercase_task.clone(), write_path, FileStamper::Modified);

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&write_task), CommonOutput::write_string_to_file_ok());
    assert!(session.dependency_check_errors().is_empty());
  });

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting ToLowerCase, but not 
  // affecting WriteStringToFile because the output from ToLowerCase does not change.
  write_until_modified(&read_path, "hello world!").check();

  pie.run_in_session(|mut session| {
    session.update_affected_by(&[read_path.clone()]);
    assert!(session.dependency_check_errors().is_empty());

    let tracker = &mut session.tracker_mut().0;
    let read_task_end = tracker.get_index_of_execute_end_of_with(&read_task, &CommonOutput::read_string_from_file_ok("hello world!"));
    assert_matches!(read_task_end, Some(_));
    let to_lowercase_task_end = tracker.get_index_of_execute_end_of_with(&to_lowercase_task, &CommonOutput::to_lower_case_ok("hello world!"));
    assert_matches!(to_lowercase_task_end, Some(_));
    assert!(to_lowercase_task_end > read_task_end);
    assert!(tracker.contains_no_execute_end_of(&write_task));
  });
}

#[rstest]
fn test_indirectly_affected_multiple_tasks(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "HELLO WORLD!").check();
  let write_lower_path = temp_dir.path().join("out_lower.txt");
  let write_upper_path = temp_dir.path().join("out_upper.txt");

  let read_task = CommonTask::read_string_from_file(&read_path, FileStamper::Modified);
  let to_lowercase_task = CommonTask::to_lower_case(read_task.clone());
  let to_uppercase_task = CommonTask::to_upper_case(read_task.clone());
  let write_lowercase_task = CommonTask::write_string_to_file(to_lowercase_task.clone(), write_lower_path.clone(), FileStamper::Modified);
  let write_uppercase_task = CommonTask::write_string_to_file(to_uppercase_task.clone(), write_upper_path.clone(), FileStamper::Modified);

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&write_lowercase_task), CommonOutput::write_string_to_file_ok());
    assert_eq!(session.require(&write_uppercase_task), CommonOutput::write_string_to_file_ok());
    assert!(session.dependency_check_errors().is_empty());
  });

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting ToLowerCase and 
  // ToUpperCase, but not their WriteStringToFile tasks.
  write_until_modified(&read_path, "hello world!").check();
  pie.run_in_session(|mut session| {
    session.update_affected_by(&[read_path.clone()]);
    assert!(session.dependency_check_errors().is_empty());

    let tracker = &mut session.tracker_mut().0;
    let read_task_end = tracker.get_index_of_execute_end_of_with(&read_task, &CommonOutput::read_string_from_file_ok("hello world!"));
    assert_matches!(read_task_end, Some(_));

    let to_lowercase_task_end = tracker.get_index_of_execute_end_of_with(&to_lowercase_task, &CommonOutput::to_lower_case_ok("hello world!"));
    assert_matches!(to_lowercase_task_end, Some(_));
    assert!(to_lowercase_task_end > read_task_end);
    assert!(tracker.contains_no_execute_end_of(&write_lowercase_task));

    let to_uppercase_task_end = tracker.get_index_of_execute_end_of_with(&to_uppercase_task, &CommonOutput::to_upper_case_ok("HELLO WORLD!"));
    assert_matches!(to_uppercase_task_end, Some(_));
    assert!(to_uppercase_task_end > read_task_end);
    assert!(tracker.contains_no_execute_end_of(&write_uppercase_task));
  });

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting all other tasks.
  write_until_modified(&read_path, "hello world!!").check();
  pie.run_in_session(|mut session| {
    session.update_affected_by(&[read_path.clone()]);
    assert!(session.dependency_check_errors().is_empty());

    let tracker = &mut session.tracker_mut().0;
    let read_task_end = tracker.get_index_of_execute_end_of_with(&read_task, &CommonOutput::read_string_from_file_ok("hello world!!"));
    assert_matches!(read_task_end, Some(_));

    let to_lowercase_task_end = tracker.get_index_of_execute_end_of_with(&to_lowercase_task, &CommonOutput::to_lower_case_ok("hello world!!"));
    assert_matches!(to_lowercase_task_end, Some(_));
    assert!(to_lowercase_task_end > read_task_end);
    let write_lowercase_task_end = tracker.get_index_of_execute_end_of(&write_lowercase_task);
    assert_matches!(write_lowercase_task_end, Some(_));
    assert!(write_lowercase_task_end > to_lowercase_task_end);
    assert_eq!(read_to_string(&write_lower_path).check(), "hello world!!".to_string());

    let to_uppercase_task_end = tracker.get_index_of_execute_end_of_with(&to_uppercase_task, &CommonOutput::to_upper_case_ok("HELLO WORLD!!"));
    assert_matches!(to_uppercase_task_end, Some(_));
    assert!(to_uppercase_task_end > read_task_end);
    let write_uppercase_task_end = tracker.get_index_of_execute_end_of(&write_uppercase_task);
    assert_matches!(write_uppercase_task_end, Some(_));
    assert!(write_uppercase_task_end > to_uppercase_task_end);
    assert_eq!(read_to_string(&write_upper_path).check(), "HELLO WORLD!!".to_string());
  });
}

#[rstest]
fn test_require_now(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let marker_path = temp_dir.path().join("marker.txt");
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "hello world!").check();

  let to_lower_task = CommonTask::to_lower_case(CommonTask::read_string_from_file(read_path.clone(), FileStamper::Modified));
  let task = CommonTask::require_task_on_file_exists(to_lower_task.clone(), marker_path.clone());

  pie.run_in_session(|mut session| {
    session.require(&to_lower_task);
    session.require(&task); // `task` does not require `to_lower_task` because `marker.txt` does not exist.
  });

  // Create the marker file, so `task` will require `to_lower_task`.
  write(&marker_path, "").check();
  // Change the file that ReadStringFromFile reads, which `to_lower_task` depends on, thus `to_lower_task` is affected and should be executed.
  write_until_modified(&read_path, "hello world!!").check();

  pie.run_in_session(|mut session| {
    session.update_affected_by(&[read_path, marker_path]);

    let tracker = &mut session.tracker_mut().0;
    let task_end = tracker.get_index_of_execute_end_of(&task);
    assert_matches!(task_end, Some(_));
    let to_lower_task_end = tracker.get_index_of_execute_end_of(&to_lower_task);
    assert_matches!(to_lower_task_end, Some(_));
    assert!(task_end > to_lower_task_end); // Ensure that `to_lower_task` finishes execution before `task`.
  });
}
