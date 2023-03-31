use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, Task};
use crate::fs::open_if_file;

pub struct NonIncrementalContext;

impl<T: Task> Context<T> for NonIncrementalContext {
  fn require_task(&mut self, task: &T) -> T::Output {
    task.execute(self)
  }
  fn require_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>, io::Error> {
    let file = open_if_file(&path)?;
    Ok(file)
  }
}
