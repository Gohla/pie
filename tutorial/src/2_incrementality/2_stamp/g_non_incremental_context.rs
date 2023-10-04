use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, Task};
use crate::fs::open_if_file;
use crate::stamp::{FileStamper, OutputStamper};

pub struct NonIncrementalContext;

impl<T: Task> Context<T> for NonIncrementalContext {
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, _stamper: FileStamper) -> Result<Option<File>, io::Error> {
    open_if_file(&path)
  }

  fn require_task_with_stamper(&mut self, task: &T, _stamper: OutputStamper) -> T::Output {
    task.execute(self)
  }
}

