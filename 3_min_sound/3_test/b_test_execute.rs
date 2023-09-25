use std::io;

use assert_matches::assert_matches;

use pie::tracker::event::Event::*;

use crate::common::{test_pie, TestPieExt, TestTask::*};

#[path = "common.rs"]
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
