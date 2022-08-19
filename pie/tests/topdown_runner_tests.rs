use std::fs;
use std::io::Stdout;

use rstest::{fixture, rstest};
use tempfile::TempDir;

use pie::prelude::*;
use pie::tracker::{CompositeTracker, Event, EventTracker, WritingTracker};

use crate::common::{CheckErrorExt, ReadStringFromFile, ToLowerCase, WriteStringToFile};

mod common;

type Runner = TopDownRunner<CompositeTracker<EventTracker, WritingTracker<Stdout>>>;

#[fixture]
fn runner() -> Runner { common::create_runner() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_exec(mut runner: Runner) {
  let task = ToLowerCase("CAPITALIZED".to_string());
  let dyn_task = task.clone_box_dyn();
  assert_eq!(runner.require_initial(&task).check(), "capitalized");
  let tracker = &runner.tracker().0;
  assert!(match tracker.get_from_end(2) {
    Some(Event::RequireTask(t)) if t == &dyn_task => true,
    _ => false,
  });
  assert!(match tracker.get_from_end(1) {
    Some(Event::ExecuteTaskStart(t)) if t == &dyn_task => true,
    _ => false,
  });
  assert!(match tracker.get_from_end(0) {
    Some(Event::ExecuteTaskEnd(t, _)) if t == &dyn_task => true,
    _ => false,
  });
}

#[rstest]
fn test_reuse(mut runner: Runner) {
  use Event::*;
  let task = ToLowerCase("CAPITALIZED".to_string());
  let dyn_task = task.clone_box_dyn();

  assert_eq!(runner.require_initial(&task).check(), "capitalized");
  {
    let tracker = &mut runner.tracker_mut().0;
    assert!(match tracker.get_from_end(0) {
      Some(ExecuteTaskEnd(t, _)) if t == &dyn_task => true,
      _ => false,
    });
    assert!(match tracker.get_from_end(1) {
      Some(ExecuteTaskStart(t)) if t == &dyn_task => true,
      _ => false,
    });
    assert!(match tracker.get_from_end(2) {
      Some(RequireTask(t)) if t == &dyn_task => true,
      _ => false,
    });
    tracker.clear();
  }

  assert_eq!(runner.require_initial(&task).check(), "capitalized");
  {
    let tracker = &mut runner.tracker_mut().0;
    dbg!(&tracker);
    assert!(!tracker.iter_events().any(|e| match e { // Assert that no executions have taken place.
      ExecuteTaskStart(_) => true,
      _ => false
    }));
  }
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn cycle_panics(mut runner: Runner) {
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct RequireSelf;
  impl Task for RequireSelf {
    type Output = ();
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      context.require_task(self);
    }
  }
  runner.require_initial(&RequireSelf).check();
}

#[rstest]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics(mut runner: Runner, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  let task_1 = WriteStringToFile(path.clone(), "Test 1".to_string());
  runner.require_initial(&task_1).check().check();
  let task_2 = WriteStringToFile(path.clone(), "Test 2".to_string());
  runner.require_initial(&task_2).check().check();
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics(mut runner: Runner, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  let providing_task = WriteStringToFile(path.clone(), "Test 1".to_string());
  runner.require_initial(&providing_task).check().check();
  let requiring_task = ReadStringFromFile(path.clone());
  runner.require_initial(&requiring_task).check().check();
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics(mut runner: Runner, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "test").check();
  let requiring_task = ReadStringFromFile(path.clone());
  runner.require_initial(&requiring_task).check().check();
  let providing_task = WriteStringToFile(path.clone(), "Test 1".to_string());
  runner.require_initial(&providing_task).check().check();
}
