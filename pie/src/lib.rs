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
