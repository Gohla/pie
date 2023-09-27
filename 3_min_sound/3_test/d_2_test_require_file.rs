use std::fs::write;
use std::io;

use assert_matches::assert_matches;

use dev_shared::{create_temp_dir, write_until_modified};
use pie::stamp::FileStamper;
use pie::tracker::event::Event::*;

use crate::common::{test_pie, TestPieExt, TestTask::*};

mod common;

#[test]
fn test_execution() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let task = StringConstant("Hello, World!");
  let output = pie.require_then_assert(&task, |tracker| {
    let events = tracker.slice();
    assert_matches!(events.get(0), Some(RequireTask { task: t, .. }) if t == &task);
    assert_matches!(events.get(1), Some(Execute { task: t }) if t == &task);
    assert_matches!(events.get(2), Some(Executed { task: t, .. }) if t == &task);
    assert_matches!(events.get(3), Some(RequiredTask { task: t, .. }) if t == &task);
  })?;
  assert_eq!(output.as_str(), "Hello, World!");
  Ok(())
}

#[test]
fn test_reuse() -> Result<(), io::Error> {
  let mut pie = test_pie();
  let task = StringConstant("Hello, World!");
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

  let path = temp_dir.path().join("in.txt");
  write(&path, "HELLO WORLD!")?;
  let task = ReadStringFromFile(path.clone(), FileStamper::Modified);
  
  // Require task and assert that it is executed because it is new.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // Require task again and assert that it is not executed because all its dependencies are consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // Change required file such that the file dependency of the task becomes inconsistent.
  write_until_modified(&path, "!DLROW OLLEH")?;
  // Require task again and assert that it re-executed because its file dependency is inconsistent.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "!DLROW OLLEH");

  // Repeat the test with FileStamper::Exists, which results in a different outcome.
  write(&path, "HELLO WORLD!")?;
  let task = ReadStringFromFile(path.clone(), FileStamper::Exists);

  // Require task and assert that it is executed because it is new.
  let output = pie.require_then_assert_one_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // Require task again and assert that it is not executed because all its dependencies are consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");
  // Change required file, but the file dependency of the task stays consistent.
  write_until_modified(&path, "!DLROW OLLEH")?;
  // Require task again and assert that it is not executed because all its dependencies are consistent.
  let output = pie.require_then_assert_no_execute(&task)?;
  assert_eq!(output.as_str(), "HELLO WORLD!");

  Ok(())
}
