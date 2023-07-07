use std::error::Error;
use std::fs::{read_to_string, write};

use assert_matches::assert_matches;
use rstest::rstest;
use tempfile::TempDir;

use ::pie::{Context, Task};
use ::pie::stamp::FileStamper;
use ::pie::tracker::event::Event::*;
use dev_shared::write_until_modified;
use dev_shared_external::task::*;
use dev_shared_external::test::*;

#[rstest]
fn test_exec(mut pie: TestPie<CommonTask>) -> Result<(), Box<dyn Error>> {
  let task = StringConstant::new("Hello, World!");
  let output = pie.require_then_assert(&task, |tracker| {
    let events = tracker.slice();
    assert_matches!(events.get(0), Some(RequireTaskStart { task: t }) if t == &task);
    assert_matches!(events.get(1), Some(ExecuteStart { task: t }) if t == &task);
    assert_matches!(events.get(2), Some(ExecuteEnd { task: t, .. }) if t == &task);
  })?;
  assert_eq!(output.as_str(), "Hello, World!");
  Ok(())
}

#[rstest]
fn test_reuse(mut pie: TestPie<CommonTask>) -> Result<(), Box<dyn Error>> {
  let task = StringConstant::new("Hello world");
  // New task: execute.
  let output = pie.require(&task)?;
  assert_eq!(output.as_str(), "Hello world");
  // Nothing changed: no execute
  pie.require_then_assert_no_execute(&task)?;
  Ok(())
}

#[rstest]
fn test_require_task(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;
  let read_task = ReadStringFromFile::new(&path, FileStamper::Modified);
  let task = ToLowerCase::new(&read_task);

  // Require task and observe that the tasks are executed in dependency order.
  let output = pie.require_then_assert(&task, |tracker| {
    let task_start = assert_matches!(tracker.index_execute_start(&task), Some(i) => i);
    let read_task_start = assert_matches!(tracker.index_execute_start(&read_task), Some(i) => i);
    let read_task_end = assert_matches!(tracker.index_execute_end(&read_task), Some(i) => i);
    assert!(read_task_end > read_task_start);
    assert!(read_task_start > task_start);
    let task_end = assert_matches!(tracker.index_execute_end(&task), Some(i) => i);
    assert!(task_end > task_start);
    assert!(task_end > read_task_end);
  })?;
  assert_eq!(output.as_str(), "hello world!");

  // Require task again and observe that no tasks are executed since they are not affected.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "hello world!");

  // Change required file such that the task is affected.
  write_until_modified(&path, "!DLROW OLLEH")?;

  // Require task and observe that all tasks are re-executed in reverse dependency order.
  let output = pie.require_then_assert(&task, |tracker| {
    let read_task_start = assert_matches!(tracker.index_execute_start(&read_task), Some(i) => i);
    let read_task_end = assert_matches!(tracker.index_execute_end(&read_task), Some(i) => i);
    let task_start = assert_matches!(tracker.index_execute_start(&task), Some(i) => i);
    let task_end = assert_matches!(tracker.index_execute_end(&task), Some(i) => i);
    assert!(task_start > read_task_start);
    assert!(task_end > read_task_end);
  })?;
  assert_eq!(output.as_str(), "!dlrow olleh");

  Ok(())
}

#[rstest]
fn test_require_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;
  let task = ReadStringFromFile::new(&path, FileStamper::Modified);

  // Require task and observe that it is executed.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");

  // Require task again and observe that it is not executed since it is not affected.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");

  // Change required file such that the task is affected.
  write_until_modified(&path, "!DLROW OLLEH")?;

  // Require task again and observe that it re-executed since it affected.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "!DLROW OLLEH");

  Ok(())
}

#[rstest]
fn test_provide_file(mut pie: TestPie<CommonTask>, temp_dir: TempDir) -> Result<(), Box<dyn Error>> {
  let path = temp_dir.path().join("out.txt");
  let task = WriteStringToFile::new(StringConstant::new("HELLO WORLD!"), &path, FileStamper::Modified);

  // Require task and observe that it is executed.
  pie.require_then_assert_one_execute(&task)?;
  assert_eq!(read_to_string(&path)?.as_str(), "HELLO WORLD!");

  // Require task again and observe that it is not executed since it is not affected.
  pie.require_then_assert_no_execute(&task)?;
  assert_eq!(read_to_string(&path)?.as_str(), "HELLO WORLD!");

  // Change provided file such that the task is affected.
  write_until_modified(&path, "!DLROW OLLEH")?;

  // Require task again and observe that it re-executed since it affected.
  pie.require_then_assert_one_execute(&task)?;
  assert_eq!(read_to_string(&path)?.as_str(), "HELLO WORLD!");

  Ok(())
}


// Cycle detection tests.

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
enum Cycle {
  RequireSelf,
  RequireA,
  RequireB,
}

impl Task for Cycle {
  type Output = ();
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    match self {
      Self::RequireSelf => context.require_task(&Self::RequireSelf),
      Self::RequireA => context.require_task(&Self::RequireB),
      Self::RequireB => context.require_task(&Self::RequireA),
    }
  }
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_panics(mut pie: TestPie<Cycle>) {
  pie.require(&Cycle::RequireSelf);
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_a_panics(mut pie: TestPie<Cycle>) {
  pie.require(&Cycle::RequireA);
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_cycle_b_panics(mut pie: TestPie<Cycle>) {
  pie.require(&Cycle::RequireB);
}


// Overlapping and hidden dependency detection tests.

#[rstest]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("out.txt");

  pie.run_in_session(|mut session| {
    let task_1 = WriteStringToFile::new(StringConstant::new("Test 1"), &path, FileStamper::Modified);
    session.require(&task_1).unwrap();
    let task_2 = WriteStringToFile::new(StringConstant::new("Test 2"), &path, FileStamper::Modified);
    session.require(&task_2).unwrap();
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("in_out.txt");

  pie.run_in_session(|mut session| {
    let providing_task = WriteStringToFile::new(StringConstant::new("Test 1"), &path, FileStamper::Modified);
    session.require(&providing_task).unwrap();
    let requiring_task = ReadStringFromFile::new(&path, FileStamper::Modified);
    session.require(&requiring_task).unwrap();
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics(mut pie: TestPie<CommonTask>, temp_dir: TempDir) {
  let path = temp_dir.path().join("in_out.txt");
  write(&path, "test").expect("failed to write to file");

  pie.run_in_session(|mut session| {
    let requiring_task = ReadStringFromFile::new(&path, FileStamper::Modified);
    session.require(&requiring_task).unwrap();
    let providing_task = WriteStringToFile::new(StringConstant::new("Test 1"), &path, FileStamper::Modified);
    session.require(&providing_task).unwrap();
  });
}
