use std::fs;

use pie::prelude::*;
use pie::tracker::Event;

use crate::common::{CheckErrorExt, create_runner, ReadStringFromFile, temp_dir, WriteStringToFile};

mod common;

#[test]
fn test() {
  let mut runner = create_runner();
  let dir = temp_dir();
  let path = dir.path().join("test.txt");
  fs::write(&path, "test").check();
  let task = ReadStringFromFile::new(path);
  let dyn_task = task.clone_box_dyn();
  assert_eq!("test", runner.require_initial(&task).check().check());
  assert!(match runner.tracker().0.last() {
    Some(Event::ExecuteTaskEnd(t, _)) if t == &dyn_task => true,
    _ => false,
  });
  runner.tracker_mut().0.clear();
  assert_eq!("test", runner.require_initial(&task).check().check());
  assert!(match runner.tracker().0.last() {
    Some(Event::RequireTask(t)) if t == &dyn_task => true,
    _ => false,
  });
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn cycle_panics() {
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct RequireSelf;
  impl Task for RequireSelf {
    type Output = ();
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      context.require_task(self);
    }
  }
  let mut runner = create_runner();
  runner.require_initial(&RequireSelf).check();
}

#[test]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics() {
  let mut runner = create_runner();
  let dir = temp_dir();
  let path = dir.path().join("test.txt");
  let task_1 = WriteStringToFile::new(path.clone(), "Test 1");
  runner.require_initial(&task_1).check().check();
  let task_2 = WriteStringToFile::new(path.clone(), "Test 2");
  runner.require_initial(&task_2).check().check();
}

#[test]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_require_panics() {
  let mut runner = create_runner();
  let dir = temp_dir();
  let path = dir.path().join("test.txt");
  let providing_task = WriteStringToFile::new(path.clone(), "Test 1");
  runner.require_initial(&providing_task).check().check();
  let requiring_task = ReadStringFromFile::new(path.clone());
  runner.require_initial(&requiring_task).check().check();
}

#[test]
#[should_panic(expected = "Hidden dependency")]
fn hidden_dependency_during_provide_panics() {
  let mut runner = create_runner();
  let dir = temp_dir();
  let path = dir.path().join("test.txt");
  fs::write(&path, "test").check();
  let requiring_task = ReadStringFromFile::new(path.clone());
  runner.require_initial(&requiring_task).check().check();
  let providing_task = WriteStringToFile::new(path.clone(), "Test 1");
  runner.require_initial(&providing_task).check().check();
}
