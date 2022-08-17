use std::fs;
use std::path::PathBuf;

use pie::Context;
use pie::runner::topdown::TopDownRunner;
use pie::task::Task;

use crate::common::{ReadStringFromFile, WriteStringToFile};

mod common;

#[test]
fn test() {
  let mut runner = TopDownRunner::new();
  let path = PathBuf::from("../target/test/test.txt");
  fs::create_dir_all(path.parent().unwrap()).unwrap();
  fs::write(&path, "test").unwrap();
  let task = ReadStringFromFile::new(path);
  runner.require_initial(&task).expect("no dependency checking errors").expect("no file read error");
  runner.require_initial(&task).expect("no dependency checking errors").expect("no file read error");
}

#[test]
#[should_panic(expected = "Cyclic task dependency")]
fn cycle_panics() {
  let mut runner = TopDownRunner::new();
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct RequireSelf;
  impl Task for RequireSelf {
    type Output = ();
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      context.require_task(self);
    }
  }
  runner.require_initial(&RequireSelf).expect("no dependency checking errors");
}

#[test]
#[should_panic(expected = "Overlapping provided file")]
fn overlapping_provided_file_panics() {
  let mut runner = TopDownRunner::new();
  let path = PathBuf::from("../target/test/test.txt");
  let task_1 = WriteStringToFile::new(path.clone(), "Test 1");
  runner.require_initial(&task_1).expect("no dependency checking errors").expect("no file write error");
  let task_2 = WriteStringToFile::new(path, "Test 2");
  runner.require_initial(&task_2).expect("no dependency checking errors").expect("no file write error");
}

