use std::error::Error;
use std::fs::{read_to_string, write};

use assert_matches::assert_matches;
use rstest::rstest;
use tempfile::TempDir;

use ::pie::stamp::FileStamper;
use dev_shared::fs::write_until_modified;
use dev_shared::task::CommonTask;
use dev_shared::test::{pie, temp_dir, TestPie, TestPieExt};

#[rstest]
fn test_nothing_affected(mut pie: TestPie<CommonTask>) {
  pie.update_affected_by_then_assert([], |tracker| {
    assert!(!tracker.any_execute());
  });
}

#[rstest]
fn test_directly_affected_task(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("test.txt");
  write(&path, "HELLO WORLD!")?;

  let task = CommonTask::read_string_from_file(&path, FileStamper::Modified);

  // Initially require the task.
  let output = pie.require(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");

  // Change the file that the task requires, directly affecting it.
  write_until_modified(&path, "hello world!")?;
  pie.update_affected_by_then_assert([&path], |tracker| {
    assert_matches!(tracker.find_execute_end(&task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!");  
    });
  });

  Ok(())
}

#[rstest]
fn test_indirectly_affected_tasks(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;

  let read_task = CommonTask::read_string_from_file(&path, FileStamper::Modified);
  let to_lowercase_task = CommonTask::to_lower_case(read_task.clone());

  // Initially require the tasks.
  let output = pie.require(&to_lowercase_task)?;
  assert_eq!(output.as_str(), "hello world!");

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting ToLowerCase, 
  // indirectly affecting CombineA.
  write_until_modified(&path, "HELLO WORLD!!")?;
  pie.update_affected_by_then_assert([&path], |tracker| {
    // ReadStringFromFile
    assert_matches!(tracker.find_execute_end(&read_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "HELLO WORLD!!");  
    });
    let read_task_end = assert_matches!(tracker.index_execute_end(&read_task), Some(p) => p);
    // ToLowerCase
    assert_matches!(tracker.find_execute_end(&to_lowercase_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!!");  
    });
    let to_lowercase_task_end = assert_matches!(tracker.index_execute_end(&to_lowercase_task), Some(p) => p);
    assert!(to_lowercase_task_end > read_task_end);
  });

  Ok(())
}

#[rstest]
fn test_indirectly_affected_tasks_early_cutoff(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "HELLO WORLD!")?;
  let write_path = temp_dir.path().join("out.txt");

  let read_task = CommonTask::read_string_from_file(&read_path, FileStamper::Modified);
  let to_lowercase_task = CommonTask::to_lower_case(read_task.clone());
  let write_task = CommonTask::write_string_to_file(to_lowercase_task.clone(), write_path, FileStamper::Modified);

  // Initially require the tasks.
  pie.require(&write_task)?;

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting ToLowerCase, but not 
  // affecting WriteStringToFile because the output from ToLowerCase does not change.
  write_until_modified(&read_path, "hello world!")?;
  pie.update_affected_by_then_assert([&read_path], |tracker| {
    // ReadStringFromFile
    assert_matches!(tracker.find_execute_end(&read_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!");  
    });
    let read_task_end = assert_matches!(tracker.index_execute_end(&read_task), Some(p) => p);
    // ToLowerCase
    assert_matches!(tracker.find_execute_end(&to_lowercase_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!");  
    });
    let to_lowercase_task_end = assert_matches!(tracker.index_execute_end(&to_lowercase_task), Some(p) => p);
    assert!(to_lowercase_task_end > read_task_end);
    // WriteStringToFile
    assert!(!tracker.any_execute_of(&write_task));
  });

  Ok(())
}

#[rstest]
fn test_indirectly_affected_multiple_tasks(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "HELLO WORLD!")?;
  let write_lower_path = temp_dir.path().join("out_lower.txt");
  let write_upper_path = temp_dir.path().join("out_upper.txt");

  let read_task = CommonTask::read_string_from_file(&read_path, FileStamper::Modified);
  let to_lowercase_task = CommonTask::to_lower_case(read_task.clone());
  let to_uppercase_task = CommonTask::to_upper_case(read_task.clone());
  let write_lowercase_task = CommonTask::write_string_to_file(to_lowercase_task.clone(), write_lower_path.clone(), FileStamper::Modified);
  let write_uppercase_task = CommonTask::write_string_to_file(to_uppercase_task.clone(), write_upper_path.clone(), FileStamper::Modified);

  // Initially require the tasks.
  pie.assert_in_session(|session| {
    session.require(&write_lowercase_task)?;
    session.require(&write_uppercase_task)
  }, |_| {})?;

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting ToLowerCase and 
  // ToUpperCase, but not their WriteStringToFile tasks.
  write_until_modified(&read_path, "hello world!")?;
  pie.update_affected_by_then_assert([&read_path], |tracker| {
    // ReadStringFromFile
    assert_matches!(tracker.find_execute_end(&read_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!");  
    });
    let read_task_end = assert_matches!(tracker.index_execute_end(&read_task), Some(p) => p);
    // ToLowerCase
    assert_matches!(tracker.find_execute_end(&to_lowercase_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!");  
    });
    let to_lowercase_task_end = assert_matches!(tracker.index_execute_end(&to_lowercase_task), Some(p) => p);
    assert!(to_lowercase_task_end > read_task_end);
    // WriteStringToFile(ToLowerCase)
    assert!(!tracker.any_execute_of(&write_lowercase_task));
    // ToUpperCase
    assert_matches!(tracker.find_execute_end(&to_uppercase_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "HELLO WORLD!");  
    });
    let to_uppercase_task_end = assert_matches!(tracker.index_execute_end(&to_uppercase_task), Some(p) => p);
    assert!(to_uppercase_task_end > read_task_end);
    // WriteStringToFile(ToUpperCase)
    assert!(!tracker.any_execute_of(&write_uppercase_task));
  });

  // Change the file that ReadStringFromFile requires, directly affecting it, indirectly affecting all other tasks.
  write_until_modified(&read_path, "hello world!!")?;
  pie.update_affected_by_then_assert([&read_path], |tracker| {
    // ReadStringFromFile
    assert_matches!(tracker.find_execute_end(&read_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!!");  
    });
    let read_task_end = assert_matches!(tracker.index_execute_end(&read_task), Some(p) => p);
    // ToLowerCase
    assert_matches!(tracker.find_execute_end(&to_lowercase_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "hello world!!");  
    });
    let to_lowercase_task_end = assert_matches!(tracker.index_execute_end(&to_lowercase_task), Some(p) => p);
    assert!(to_lowercase_task_end > read_task_end);
    // WriteStringToFile(ToLowerCase)
    let write_lowercase_task_end = assert_matches!(tracker.index_execute_end(&write_lowercase_task), Some(p) => p);
    assert!(write_lowercase_task_end > to_lowercase_task_end);
    // ToUpperCase
    assert_matches!(tracker.find_execute_end(&to_uppercase_task), Some(Ok(output)) => {
      assert_eq!(output.as_str(), "HELLO WORLD!!");  
    });
    let to_uppercase_task_end = assert_matches!(tracker.index_execute_end(&to_uppercase_task), Some(p) => p);
    assert!(to_uppercase_task_end > read_task_end);
    // WriteStringToFile(ToUpperCase)
    let write_uppercase_task_end = assert_matches!(tracker.index_execute_end(&write_uppercase_task), Some(p) => p);
    assert!(write_uppercase_task_end > to_uppercase_task_end);
  });
  assert_eq!(read_to_string(&write_lower_path)?.as_str(), "hello world!!");
  assert_eq!(read_to_string(&write_upper_path)?.as_str(), "HELLO WORLD!!");

  Ok(())
}

#[rstest]
fn test_require_now(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let marker_path = temp_dir.path().join("marker.txt");
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "hello world!")?;

  let to_lower_task = CommonTask::to_lower_case(CommonTask::read_string_from_file(read_path.clone(), FileStamper::Modified));
  let task = CommonTask::require_task_on_file_exists(to_lower_task.clone(), marker_path.clone());

  // Initially require the tasks.
  pie.assert_in_session(|session| {
    session.require(&to_lower_task)?;
    session.require(&task) // `task` does not require `to_lower_task` because `marker.txt` does not exist.
  }, |_| {})?;

  // Create the marker file, so `task` will require `to_lower_task`.
  write(&marker_path, "")?;
  // Change the file that ReadStringFromFile reads, which `to_lower_task` depends on, thus `to_lower_task` is affected and should be executed.
  write_until_modified(&read_path, "hello world!!")?;
  pie.update_affected_by_then_assert(&[read_path, marker_path], |tracker| {
    let task_end = assert_matches!(tracker.index_execute_end(&task), Some(p) => p);
    let to_lower_task_end = assert_matches!(tracker.index_execute_end(&to_lower_task), Some(p) => p);
    assert!(task_end > to_lower_task_end); // Ensure that `to_lower_task` finishes execution before `task`.
  });

  Ok(())
}
