use std::fs;

use assert_matches::assert_matches;
use rstest::{fixture, rstest};
use tempfile::TempDir;

use ::pie::prelude::*;
use ::pie::tracker::Event;

use crate::common::{CheckErrorExt, Pie, ReadStringFromFile, ToLowerCase, WriteStringToFile};

mod common;

#[fixture]
fn pie() -> Pie { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_exec(mut pie: Pie) {
  use Event::*;
  let task = ToLowerCase("CAPITALIZED".to_string());
  let dyn_task = task.clone_box_dyn();

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), "capitalized");
    assert_eq!(session.dependency_check_errors().len(), 0);
  });

  let tracker = &pie.tracker().0;
  assert_matches!(tracker.get_from_end(0), Some(ExecuteTaskEnd(t, _)) => {
    assert_eq!(t, &dyn_task);
  });
  assert_matches!(tracker.get_from_end(1), Some(ExecuteTaskStart(t)) => {
    assert_eq!(t, &dyn_task);
  });
  assert_matches!(tracker.get_from_end(2), Some(RequireTask(t)) => {
    assert_eq!(t, &dyn_task);
  });
}

#[rstest]
fn test_reuse(mut pie: Pie) {
  use Event::*;
  let task = ToLowerCase("CAPITALIZED".to_string());
  let dyn_task = task.clone_box_dyn();

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), "capitalized");
    assert_eq!(session.dependency_check_errors().len(), 0);
  });

  {
    let tracker = &mut pie.tracker_mut().0;
    assert_matches!(tracker.get_from_end(0), Some(ExecuteTaskEnd(t, _)) => {
      assert_eq!(t, &dyn_task);
    });
    assert_matches!(tracker.get_from_end(1), Some(ExecuteTaskStart(t)) => {
      assert_eq!(t, &dyn_task);
    });
    assert_matches!(tracker.get_from_end(2), Some(RequireTask(t)) => {
      assert_eq!(t, &dyn_task);
    });
    tracker.clear();
  }

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), "capitalized");
    assert_eq!(session.dependency_check_errors().len(), 0);
  });

  {
    let tracker = &mut pie.tracker_mut().0;
    assert!(!tracker.iter_events().any(|e| match e { // Assert that no executions have taken place.
      ExecuteTaskStart(_) => true,
      _ => false
    }));
  }
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_cycle_panics(mut pie: Pie) {
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct RequireSelf;
  impl Task for RequireSelf {
    type Output = ();
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      context.require_task(self);
    }
  }
  pie.run_in_session(|mut session| {
    session.require(&RequireSelf);
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics(mut pie: Pie, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  pie.run_in_session(|mut session| {
    let task_1 = WriteStringToFile(path.clone(), "Test 1".to_string());
    session.require(&task_1).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let task_2 = WriteStringToFile(path.clone(), "Test 2".to_string());
    session.require(&task_2).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics(mut pie: Pie, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  pie.run_in_session(|mut session| {
    let providing_task = WriteStringToFile(path.clone(), "Test 1".to_string());
    session.require(&providing_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let requiring_task = ReadStringFromFile(path.clone());
    session.require(&requiring_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}

#[rstest]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics(mut pie: Pie, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "test").check();
  pie.run_in_session(|mut session| {
    let requiring_task = ReadStringFromFile(path.clone());
    session.require(&requiring_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
    let providing_task = WriteStringToFile(path.clone(), "Test 1".to_string());
    session.require(&providing_task).check();
    assert_eq!(session.dependency_check_errors().len(), 0);
  });
}
