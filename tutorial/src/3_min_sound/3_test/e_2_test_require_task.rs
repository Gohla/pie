use std::fs::write;
use std::io;

use assert_matches::assert_matches;

use dev_shared::{create_temp_dir, write_until_modified};
use pie::stamp::FileStamper;
use pie::tracker::event::*;

use crate::common::{test_pie, TestPieExt, TestTask::*};

mod common;

#[test]
fn test_execution() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let task = Return("Hello, World!");
  let output = pie.require_then_assert(&task, |tracker| {
    let events = tracker.slice();
    assert_matches!(events.get(0), Some(Event::RequireTaskStart(RequireTaskStart { task: t, .. })) if t == &task);
    assert_matches!(events.get(1), Some(Event::ExecuteStart(ExecuteStart { task: t, .. })) if t == &task);
    assert_matches!(events.get(2), Some(Event::ExecuteEnd(ExecuteEnd { task: t, .. })) if t == &task);
    assert_matches!(events.get(3), Some(Event::RequireTaskEnd(RequireTaskEnd { task: t, .. })) if t == &task);
  })?;
  assert_eq!(output.as_str(), "Hello, World!");
  Ok(())
}

#[test]
fn test_reuse() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let task = Return("Hello, World!");
  // New task: execute.
  let output = pie.require(&task)?;
  assert_eq!(output.as_str(), "Hello, World!");
  // Nothing changed: no execute
  pie.require_then_assert_no_execute(&task)?;
  Ok(())
}

#[test]
fn test_require_file() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in.txt");
  write(&file, "HELLO WORLD!")?;
  let task = ReadFile(file.clone(), FileStamper::Modified);

  // 1) Require task and assert that it is executed because it is new.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // 2) Require task again and assert that it is not executed because its file dependency consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // 3) Change required file such that the file dependency of the task becomes inconsistent.
  write_until_modified(&file, "!DLROW OLLEH")?;
  // 4) Require task again and assert that it is re-executed because its file dependency is inconsistent.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "!DLROW OLLEH");

  // Repeat the test with `FileStamper::Exists`, which results in a different outcome.
  write(&file, "HELLO WORLD!")?;
  let task = ReadFile(file.clone(), FileStamper::Exists);

  // 1) Require task and assert that it is executed because it is new.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // 2) Require task again and assert that it is not executed because its file dependency is consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // 3) Change required file, but the file dependency of the task stays consistent.
  write_until_modified(&file, "!DLROW OLLEH")?;
  // 4) Require task again and assert that it is not executed because its file dependency is still consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");

  Ok(())
}

#[test]
fn test_require_task() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let temp_dir = create_temp_dir()?;

  let file = temp_dir.path().join("in.txt");
  write(&file, "HELLO WORLD!")?;
  let read = ReadFile(file.clone(), FileStamper::Modified);
  let lower = ToLower(Box::new(read.clone()));

  Ok(())
}
