use std::fs::File;
use std::path::PathBuf;

use crate::{Context, Task};

/// Non-incremental runner that ignores all dependencies and just executes tasks.
pub struct NaiveRunner {}

impl Context for NaiveRunner {
  #[inline]
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output { task.execute(self) }
  #[inline]
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> { File::open(path) }
  #[inline]
  fn provide_file(&mut self, _path: &PathBuf) -> Result<(), std::io::Error> { Ok(()) }
}
