use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, Task};
use crate::dependency::FileDependency;
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::{Store, TaskNode};

pub struct TopDownContext<T, O> {
  store: Store<T, O>,
}

impl<T: Task> TopDownContext<T, T::Output> {
  pub fn new(store: Store<T, T::Output>) -> Self {
    Self { store }
  }

  pub fn require(&mut self, task: &T) -> T::Output {
    let output = self.require_task(task);
    output
  }
}

impl<T: Task> Context<T> for TopDownContext<T, T::Output> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    let node = self.store.get_or_create_task_node(task);

    todo!("figure out what the currently executing task is");
    todo!("add task dependency to store");

    let output = if self.should_execute_task(&node, task) {
      let output = task.execute(self);
      self.store.set_task_output(&node, output);
    } else {
      self.store.get_task_output(&node).clone()
    };

    output
  }

  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    let path = path.as_ref();
    let (dependency, file) = FileDependency::new(path, stamper)?;
    let node = self.session.store.get_or_create_file_node(path);
    todo!("figure out what the currently executing task is");
    todo!("add file dependency to store");
    Ok(file)
  }
}

impl<T: Task> TopDownContext<T, T::Output> {
  fn should_execute_task(&mut self, node: &TaskNode, task: &T) -> bool {
    todo!("figure out if we should execute the task");
  }
}
