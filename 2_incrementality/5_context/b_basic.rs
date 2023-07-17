use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, fs, Task};
use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::{Store, TaskNode};

pub struct TopDownContext<T, O> {
  store: Store<T, O>,
}

impl<T: Task> TopDownContext<T, T::Output> {
  pub fn new() -> Self {
    Self {
      store: Store::default(),
    }
  }
}

impl<T: Task> Context<T> for TopDownContext<T, T::Output> {
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    todo!()
  }

  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    todo!()
  }
}
