use std::fs::File;
use std::io::Error;
use std::path::PathBuf;

use crate::{Context, Task};
use crate::stamp::{FileStamper, OutputStamper};

pub struct NaiveContext {}

impl<T: Task> Context<T> for NaiveContext {
  #[inline]
  fn require_task(&mut self, task: &T) -> T::Output {
    task.execute(self)
  }
  #[inline]
  fn require_task_with_stamper(&mut self, task: &T, _stamper: OutputStamper) -> T::Output {
    self.require_task(task)
  }

  #[inline]
  fn require_file_with_stamper(&mut self, path: &PathBuf, _stamper: FileStamper) -> Result<Option<File>, Error> {
    let exists = path.try_exists()?;
    let file = if exists {
      Some(File::open(&path)?)
    } else {
      None
    };
    Ok(file)
  }
  #[inline]
  fn provide_file_with_stamper(&mut self, _path: &PathBuf, _stamper: FileStamper) -> Result<(), Error> { Ok(()) }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }
}
