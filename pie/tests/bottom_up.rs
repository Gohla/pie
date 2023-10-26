use std::fs::{read_to_string, write};
use std::path::PathBuf;

use assert_matches::assert_matches;
use testresult::TestResult;

use dev_ext::task::*;
use dev_util::{create_temp_dir, write_until_modified};
use pie::{Context, Task};
use pie::resource::file::{ExistsChecker, FsError};
use pie::task::AlwaysConsistent;
use pie::trait_object::ValueObj;

use crate::util::{new_test_pie, TestPieExt};

mod util;

/// Downcast trait objects to references.
trait ObjExt {
  fn as_str(&self) -> &'static str;
}
impl ObjExt for Box<dyn ValueObj> {
  fn as_str(&self) -> &'static str {
    self.as_ref().as_any().downcast_ref::<&'static str>().expect("expected `&'static str`")
  }
}


#[test]
fn test_nothing_affected() {
  let mut pie = new_test_pie();

  pie.bottom_up_build_then_assert(|_| {}, |tracker| {
    assert!(!tracker.any_execute());
  });
}

#[test]
fn test_directly_affected_task() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let path = temp_dir.path().join("test.txt");
  write(&path, "HELLO WORLD!")?;
  let task = ReadFile::new(&path);

  // Initially require the task.
  let output = pie.require(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");

  // Change the file that the task requires, directly affecting it.
  write_until_modified(&path, "hello world!")?;
  pie.bottom_up_build_then_assert(|b| b.changed_resource(&path), |tracker| {
    assert_matches!(tracker.first_execute_end(&task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!");
    });
  });

  Ok(())
}

#[test]
fn test_indirectly_affected_tasks() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;
  let read_task = ReadFile::new(&path);
  let to_lowercase_task = ToLower::from(&read_task);

  // Initially require the tasks.
  let output = pie.require(&to_lowercase_task)?;
  assert_eq!(output.as_str(), "hello world!");

  // Change the file that ReadFile requires, directly affecting it, indirectly affecting ToLower,
  // indirectly affecting CombineA.
  write_until_modified(&path, "HELLO WORLD!!")?;
  pie.bottom_up_build_then_assert(|b| b.changed_resource(&path), |tracker| {
    // ReadFile
    let read_task_end = assert_matches!(tracker.first_execute_end(&read_task), Some(d) => {
      assert_eq!(d.output.as_str(), "HELLO WORLD!!");
      d.index
    });
    // ToLower
    let to_lowercase_task_end = assert_matches!(tracker.first_execute_end(&to_lowercase_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!!");
      d.index
    });
    assert!(to_lowercase_task_end > read_task_end);
  });

  Ok(())
}

#[test]
fn test_indirectly_affected_tasks_early_cutoff() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "HELLO WORLD!")?;
  let write_path = temp_dir.path().join("out.txt");
  let read_task = ReadFile::new(&read_path);
  let to_lowercase_task = ToLower::from(&read_task);
  let write_task = WriteFile::from(&to_lowercase_task, &write_path);

  // Initially require the tasks.
  pie.require(&write_task)?;

  // Change the file that ReadFile requires, directly affecting it, indirectly affecting ToLower, but not
  // affecting WriteFile because the output from ToLower does not change.
  write_until_modified(&read_path, "hello world!")?;
  pie.bottom_up_build_then_assert(|b| b.changed_resource(&read_path), |tracker| {
    // ReadFile
    let read_task_end = assert_matches!(tracker.first_execute_end(&read_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!");
      d.index
    });
    // ToLower
    let to_lowercase_task_end = assert_matches!(tracker.first_execute_end(&to_lowercase_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!");
      d.index
    });
    assert!(to_lowercase_task_end > read_task_end);
    // WriteFile
    assert!(!tracker.any_execute_of(&write_task));
  });

  Ok(())
}

#[test]
fn test_indirectly_affected_multiple_tasks() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "HELLO WORLD!")?;
  let write_lower_path = temp_dir.path().join("out_lower.txt");
  let write_upper_path = temp_dir.path().join("out_upper.txt");
  let read_task = ReadFile::new(&read_path);
  let to_lowercase_task = ToLower::from(&read_task);
  let to_uppercase_task = ToUpper::from(&read_task);
  let write_lowercase_task = WriteFile::from(&to_lowercase_task, &write_lower_path);
  let write_uppercase_task = WriteFile::from(&to_uppercase_task, &write_upper_path);

  // Initially require the tasks.
  pie.assert_in_session(|session| {
    session.require(&write_lowercase_task)?;
    session.require(&write_uppercase_task)
  }, |_| {})?;

  // Change the file that ReadFile requires, directly affecting it, indirectly affecting ToLower and
  // ToUpper, but not their WriteFile tasks.
  write_until_modified(&read_path, "hello world!")?;
  pie.bottom_up_build_then_assert(|b| b.changed_resource(&read_path), |tracker| {
    // ReadFile
    let read_task_end = assert_matches!(tracker.first_execute_end(&read_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!");
      d.index
    });
    // ToLower
    let to_lowercase_task_end = assert_matches!(tracker.first_execute_end(&to_lowercase_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!");
      d.index
    });
    assert!(to_lowercase_task_end > read_task_end);
    // WriteFile(ToLower)
    assert!(!tracker.any_execute_of(&write_lowercase_task));
    // ToUpper
    let to_uppercase_task_end = assert_matches!(tracker.first_execute_end(&to_uppercase_task), Some(d) => {
      assert_eq!(d.output.as_str(), "HELLO WORLD!");
      d.index
    });
    assert!(to_uppercase_task_end > read_task_end);
    // WriteFile(ToUpper)
    assert!(!tracker.any_execute_of(&write_uppercase_task));
  });

  // Change the file that ReadFile requires, directly affecting it, indirectly affecting all other tasks.
  write_until_modified(&read_path, "hello world!!")?;
  pie.bottom_up_build_then_assert(|b| b.changed_resource(&read_path), |tracker| {
    // ReadFile
    let read_task_end = assert_matches!(tracker.first_execute_end(&read_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!!");
      d.index
    });
    // ToLower
    let to_lowercase_task_end = assert_matches!(tracker.first_execute_end(&to_lowercase_task), Some(d) => {
      assert_eq!(d.output.as_str(), "hello world!!");
      d.index
    });
    assert!(to_lowercase_task_end > read_task_end);
    // WriteFile(ToLower)
    let write_lowercase_task_end = assert_matches!(tracker.first_execute_end_index(&write_lowercase_task), Some(i) => i);
    assert!(*write_lowercase_task_end > to_lowercase_task_end);
    // ToUpper
    let to_uppercase_task_end = assert_matches!(tracker.first_execute_end(&to_uppercase_task), Some(d) => {
      assert_eq!(d.output.as_str(), "HELLO WORLD!!");
      d.index
    });
    assert!(to_uppercase_task_end > read_task_end);
    // WriteFile(ToUpper)
    let write_uppercase_task_end = assert_matches!(tracker.first_execute_end_index(&write_uppercase_task), Some(i) => i);
    assert!(*write_uppercase_task_end > to_uppercase_task_end);
  });
  assert_eq!(read_to_string(&write_lower_path)?.as_str(), "hello world!!");
  assert_eq!(read_to_string(&write_upper_path)?.as_str(), "HELLO WORLD!!");

  Ok(())
}


/// Require a task only when a file exists.
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct RequireWhenFileExists<T>(T, PathBuf);
impl<T: Task> RequireWhenFileExists<T> {
  pub fn from(task: &T, file: impl Into<PathBuf>) -> RequireWhenFileExists<T> {
    Self(task.clone(), file.into())
  }
}
impl<T: Task> Task for RequireWhenFileExists<T> {
  type Output = Result<(), FsError>;

  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    if context.read(&self.1, ExistsChecker)?.is_file() {
      // HACK: use AlwaysConsistent to ignore result, but the error of the task may influence us!
      context.require(&self.0, AlwaysConsistent);
    }
    Ok(())
  }
}

#[test]
fn test_require_now() -> TestResult {
  let mut pie = new_test_pie();
  let temp_dir = create_temp_dir()?;

  let marker_path = temp_dir.path().join("marker.txt");
  let read_path = temp_dir.path().join("in.txt");
  write(&read_path, "hello world!")?;
  let to_lower_task = ToLower::new(ReadFile::new(&read_path));
  let task = RequireWhenFileExists::from(&to_lower_task, &marker_path);

  // Initially require the tasks.
  pie.assert_in_session(|session| {
    session.require(&to_lower_task)?;
    session.require(&task) // `task` does not require `to_lower_task` because `marker.txt` does not exist.
  }, |_| {})?;

  // Create the marker file, so `task` will require `to_lower_task`.
  write(&marker_path, "")?;
  // Change the file that ReadFile reads, which `to_lower_task` depends on, thus `to_lower_task` is affected and should be executed.
  write_until_modified(&read_path, "hello world!!")?;
  pie.bottom_up_build_then_assert(|b| {
    b.changed_resource(&read_path);
    b.changed_resource(&marker_path)
  }, |tracker| {
    let task_end = assert_matches!(tracker.first_execute_end_index(&task), Some(i) => i);
    let to_lower_task_end = assert_matches!(tracker.first_execute_end_index(&to_lower_task), Some(i) => i);
    assert!(task_end > to_lower_task_end); // Ensure that `to_lower_task` finishes execution before `task`.
  });

  Ok(())
}
