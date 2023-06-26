use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::path::Path;

use stamp::{FileStamper, OutputStamper};

pub mod stamp;
pub mod context;
mod fs;
mod dependency;
mod store;

pub trait Task: Clone + Eq + Hash + Debug {
  type Output: Clone + Eq + Debug;
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}

pub trait Context<T: Task> {
  fn require_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>, io::Error> {
    self.require_file_with_stamper(path, self.default_require_file_stamper())
  }
  fn require_task(&mut self, task: &T) -> T::Output {
    self.require_task_with_stamper(task, self.default_output_stamper())
  }

  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error>;
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output;

  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
}
