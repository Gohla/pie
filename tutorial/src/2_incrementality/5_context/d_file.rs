use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, fs, Task};
use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::{Store, TaskNode};

pub struct TopDownContext<T, O> {
  store: Store<T, O>,
  current_executing_task: Option<TaskNode>,
}

impl<T: Task> TopDownContext<T, T::Output> {
  pub fn new() -> Self {
    Self {
      store: Store::default(),
      current_executing_task: None,
    }
  }
}

impl<T: Task> Context<T> for TopDownContext<T, T::Output> {
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    let Some(current_executing_task_node) = &self.current_executing_task else {
      return fs::open_if_file(path); // No current executing task, so no dependency needs to be made.
    };
    let path = path.as_ref();
    let node = self.store.get_or_create_file_node(path);
    let (dependency, file) = FileDependency::new_with_file(path, stamper)?;
    self.store.add_file_require_dependency(current_executing_task_node, &node, dependency);
    Ok(file)
  }

  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    todo!()
  }
}
