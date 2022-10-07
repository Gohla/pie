use std::fs;

use assert_matches::assert_matches;
use rstest::{fixture, rstest};
use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use ::pie::prelude::*;
use ::pie::tracker::Event;
use ::pie::trait_object::DynTaskExt;
use Event::*;

use crate::common::{CheckErrorExt, Pie, ReadStringFromFile, ToLowerCase, WriteStringToFile};

mod common;

#[fixture]
fn pie() -> Pie { common::create_pie() }

#[fixture]
fn temp_dir() -> TempDir { common::temp_dir() }


#[rstest]
fn test_exec(mut pie: Pie) {
  let task = ToLowerCase("CAPITALIZED".to_string());
  let dyn_task = task.clone_box();

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), "capitalized");
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
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
  });
}

#[rstest]
fn test_reuse(mut pie: Pie) {
  let task = ToLowerCase("CAPITALIZED".to_string());
  let dyn_task = task.clone_box();

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), "capitalized");
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
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
  });

  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), "capitalized");
    assert_eq!(session.dependency_check_errors().len(), 0);

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear()
  });
}


#[rstest]
fn test_require_task(mut pie: Pie, temp_dir: TempDir) {
  #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
  struct Combine(ReadStringFromFile);
  impl Task for Combine {
    type Output = Result<String, ()>;
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      let text = context.require_task(&self.0)?;
      Ok(context.require_task(&ToLowerCase(text)))
    }
  }
  register_task!(Combine);

  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!").check();

  let read_task = ReadStringFromFile(path.clone());
  let read_task_dyn = read_task.clone_box();
  let task = Combine(read_task);
  let dyn_task = task.clone_box();

  // Require task and observe that all three tasks are executed in dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), Ok("hello world!".to_string()));

    let tracker = &mut session.tracker_mut().0;

    let task_start = tracker.get_index_of_execute_start_of(&dyn_task);
    assert_matches!(task_start, Some(_));
    let task_end = tracker.get_index_of_execute_end_of(&dyn_task);
    assert_matches!(task_end, Some(_));
    assert!(task_start < task_end);

    let read_task_start = tracker.get_index_of_execute_start_of(&read_task_dyn);
    assert_matches!(read_task_start, Some(_));
    let read_task_end = tracker.get_index_of_execute_end_of(&read_task_dyn);
    assert_matches!(read_task_end, Some(_));
    assert!(read_task_start > task_start);

    let to_lowercase_task_dyn = ToLowerCase("HELLO WORLD!".to_string()).clone_box();
    let to_lowercase_task_start = tracker.get_index_of_execute_start_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_start, Some(_));
    let to_lowercase_task_end = tracker.get_index_of_execute_end_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_end, Some(_));
    assert!(to_lowercase_task_start < to_lowercase_task_end);
    assert!(to_lowercase_task_start > task_start);
    assert!(to_lowercase_task_start > read_task_start);

    assert!(task_end > read_task_end);
    assert!(task_end > to_lowercase_task_end);

    tracker.clear();
  });

  // Require task again and observe that no tasks are executed since they are not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), Ok("hello world!".to_string()));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Change required file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task and observe that all three tasks are re-executed in reverse dependency order
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), Ok("!dlrow olleh".to_string()));

    let tracker = &mut session.tracker_mut().0;

    let read_task_start = tracker.get_index_of_execute_start_of(&read_task_dyn);
    assert_matches!(read_task_start, Some(_));
    let read_task_end = tracker.get_index_of_execute_end_of(&read_task_dyn);
    assert_matches!(read_task_end, Some(_));

    // Old ToLowerCase task was not executed
    assert!(tracker.contains_no_execute_start_of(&ToLowerCase("HELLO WORLD!".to_string()).clone_box()));

    let to_lowercase_task_dyn = ToLowerCase("!DLROW OLLEH".to_string()).clone_box();
    let to_lowercase_task_start = tracker.get_index_of_execute_start_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_start, Some(_));
    let to_lowercase_task_end = tracker.get_index_of_execute_end_of(&to_lowercase_task_dyn);
    assert_matches!(to_lowercase_task_end, Some(_));

    let task_start = tracker.get_index_of_execute_start_of(&dyn_task);
    assert_matches!(task_start, Some(_));
    let task_end = tracker.get_index_of_execute_end_of(&dyn_task);
    assert_matches!(task_end, Some(_));

    assert!(read_task_end < to_lowercase_task_end);
    assert!(task_end > read_task_end);
    assert!(task_end > to_lowercase_task_end);

    tracker.clear();
  });

  // TODO: once stampers are implemented, only change the modification date such that ReadStringFromFile re-executes but
  //       the other tasks do not, as ReadStringFromFile still returns the same value.
}

#[rstest]
fn test_require_file(mut pie: Pie, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  fs::write(&path, "HELLO WORLD!").check();
  let task = ReadStringFromFile(path.clone());

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), Ok("HELLO WORLD!".to_string()));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), Ok("HELLO WORLD!".to_string()));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Change required file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    assert_eq!(session.require(&task), Ok("!DLROW OLLEH".to_string()));

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });
}

#[rstest]
fn test_provide_file(mut pie: Pie, temp_dir: TempDir) {
  let path = temp_dir.path().join("test.txt");
  let task = WriteStringToFile(path.clone(), "HELLO WORLD!".to_string());

  // Require task and observe that it is executed.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });

  // Require task again and observe that it is not executed since it is not affected.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_no_execute_start());
    tracker.clear();
  });

  // Change provided file such that the task is affected.
  fs::write(&path, "!DLROW OLLEH").check();

  // Require task again and observe that it re-executed since it affected.
  pie.run_in_session(|mut session| {
    session.require(&task).check();
    assert_eq!(fs::read_to_string(&path).check(), "HELLO WORLD!".to_string());

    let tracker = &mut session.tracker_mut().0;
    assert!(tracker.contains_one_execute_start());
    tracker.clear();
  });
}

#[rstest]
#[should_panic(expected = "Cyclic task dependency")]
fn require_self_cycle_panics(mut pie: Pie) {
  #[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
  struct RequireSelf;
  impl Task for RequireSelf {
    type Output = ();
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      context.require_task(self);
    }
  }
  register_task!(RequireSelf);
  
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
