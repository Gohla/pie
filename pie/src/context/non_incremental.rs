use std::fs::File;
use std::io::Error;
use std::path::Path;

use crate::{Context, Task};
use crate::fs::open_if_file;
use crate::stamp::{FileStamper, OutputStamper};

pub struct NonIncrementalContext;

impl<T: Task> Context<T> for NonIncrementalContext {
  #[inline]
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, _stamper: FileStamper) -> Result<Option<File>, Error> {
    let file = open_if_file(&path)?;
    Ok(file)
  }
  #[inline]
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, _path: P, _stamper: FileStamper) -> Result<(), Error> { Ok(()) }

  #[inline]
  fn require_task(&mut self, task: &T) -> T::Output {
    task.execute(self)
  }
  #[inline]
  fn require_task_with_stamper(&mut self, task: &T, _stamper: OutputStamper) -> T::Output {
    self.require_task(task)
  }

  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
}
