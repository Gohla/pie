use std::fs::File;
use std::path::PathBuf;

use dependency::{Dependency, FileDependency, TaskDependency};
use task::{DynTask, Task};

pub mod task;
pub mod dependency;
pub mod runner;

/// Incremental context, mediating between tasks and runners, enabling tasks to dynamically create dependencies that 
/// runners check for consistency and use in incremental execution.
pub trait Context {
  /// Requires given `[task]`, creating a dependency to it, and returning its up-to-date output.
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output;
  /// Requires file at given `[path]`, creating a read-dependency to the file by reading its content or metadata at the 
  /// time this function is called, and returning the opened file. Call this method *before reading from the file*.
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error>;
  /// Provides file at given `[path]`, creating a write-dependency to it by reading its content or metadata at the time 
  /// this function is called. Call this method *after writing to the file*.
  fn provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error>;
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
      let mut file = context.require_file(&self.path).map_err(|e| e.kind())?;
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
