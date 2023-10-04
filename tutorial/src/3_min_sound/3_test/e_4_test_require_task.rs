use std::fs::write;
use std::io;
use std::ops::RangeInclusive;

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

  // 1) Require `ToLower` and assert that both tasks are executed in dependency order, because both tasks are new:
  // → ToLower
  //   ▶ ToLower [reason: new]
  //     → ReadFile
  //       ▶ ReadFile [reason: new]
  //         - `file`
  //       ◀ Ok(String("HELLO WORLD!"))
  //     ← Ok(String("HELLO WORLD!"))
  //   ◀ Ok(String("hello world!"))
  // ← Ok(String("hello world!"))
  // 🏁
  let output = pie.require_then_assert(&lower, |tracker| {
    // `ToLower` is required and executed, and its require and execute are temporally sound.
    let lower_require = assert_matches!(tracker.first_require_task_range(&lower), Some(r) => r);
    let lower_execute = assert_matches!(tracker.first_execute_range(&lower), Some(r) => r);
    assert_task_temporally_sound(&lower_require, &lower_execute);

    // `ReadFile` is required and executed, and its require and execute are temporally sound.
    let read_require = assert_matches!(tracker.first_require_task_range(&read), Some(r) => r);
    let read_execute = assert_matches!(tracker.first_execute_range(&read), Some(r) => r);
    assert_task_temporally_sound(&read_require, &read_execute);

    // Sanity check: `file` is required.
    let file_require = assert_matches!(tracker.first_require_file_index(&file), Some(i) => i);

    // `ReadFile` is required while `ToLower` is being required.
    assert!(read_require.start() > lower_require.start());
    assert!(lower_require.end() > read_require.end());

    // `ReadFile` is executed while `ToLower` is being executed.
    assert!(read_execute.start() > lower_execute.start());
    assert!(lower_execute.end() > read_execute.end());

    // Sanity check: `ReadFile` requires `file` while executing.
    assert!(file_require > read_execute.start());
    assert!(read_execute.end() > file_require);
  })?;
  assert_eq!(output.as_str(), "hello world!");

  // 2) Require `ToLower` again and assert that no tasks are executed because all dependencies are consistent:
  // → ToLower
  //   ? ReadFile
  //     → ReadFile
  //       ✓ `file`
  //     ← Ok(String("HELLO WORLD!"))
  //   ✓ ReadFile
  // ← Ok(String("hello world!"))
  // 🏁
  let output = pie.require_then_assert_no_execute(&lower)?;
  assert_eq!(output.as_str(), "hello world!");

  Ok(())
}

/// Assert that task requires and executes are temporally sound.
fn assert_task_temporally_sound(require: &RangeInclusive<usize>, execute: &RangeInclusive<usize>) {
  // Require and execute ends come after require and execute starts.
  assert!(require.end() > require.start());
  assert!(execute.end() > execute.start());
  // A task is only executed if it is required.
  // - Task execute starts should be later than their requires. 
  assert!(execute.start() > require.start());
  // - Task require ends should be later than their executes. 
  assert!(require.end() > execute.end());
}
