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
    let node = self.store.get_or_create_task_node(task);

    // Get required task output by executing it if needed, or by getting the output from the store if not needed.
    let output = if self.should_execute_task(&node) {
      self.store.reset_task(&node);
      let previous_executing_task = self.current_executing_task.replace(node);
      let output = task.execute(self);
      self.current_executing_task = previous_executing_task;
      self.store.set_task_output(&node, output.clone());
      output
    } else {
      // Correctness: when `should_execute_task` returns `true`, the above block is executed. Otherwise this block is
      // executed and `should_execute_task` ensures that the task has an output.
      self.store.get_task_output(&node).clone()
    };

    output
  }
}
