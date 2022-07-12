use std::fs::File;
use std::path::PathBuf;

use dependency::{Dependency, FileDependency, TaskDependency};
use task::{DynTask, Task};

pub mod task;
pub mod dependency;
pub mod runner;

/// Incremental context, mediating between tasks and runners, enabling tasks to dynamically create dependencies that 
/// runners incrementally execute.
pub trait Context {
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output;
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error>;
  fn provide_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error>;
}

#[cfg(test)]
mod test {
  use std::fs;
  use std::io::Read;
  use std::path::PathBuf;

  use crate::Context;
  use crate::runner::topdown::TopDownRunner;
  use crate::task::Task;

  #[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
  pub struct ReadFileToString {
    path: PathBuf,
  }

  impl ReadFileToString {
    pub fn new(path: PathBuf) -> Self { Self { path } }
  }

  impl Task for ReadFileToString {
    // Use ErrorKind instead of Error which impls Eq and Clone.
    type Output = Result<String, std::io::ErrorKind>;
    #[inline]
    fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
      println!("Executing {:?}", self);
      let mut file = context.require_file(&self.path)?;
      let mut string = String::new();
      file.read_to_string(&mut string).map_err(|e| e.kind())?;
      Ok(string)
    }
  }

  #[test]
  fn test() {
    let mut runner = TopDownRunner::new();
    let path = PathBuf::from("../target/test/test.txt");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "test").unwrap();
    let task = ReadFileToString::new(path);
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
}
