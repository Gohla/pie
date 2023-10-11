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
  dependency_check_errors: Vec<io::Error>,
}

impl<T: Task> TopDownContext<T, T::Output> {
  pub fn new() -> Self {
    Self {
      store: Store::default(),
      current_executing_task: None,
      dependency_check_errors: Vec::default(),
    }
  }

  pub fn get_dependency_check_errors(&self) -> impl Iterator<Item=&io::Error> {
    self.dependency_check_errors.iter()
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

    // Create task require dependency if a task is currently executing (i.e., we are not requiring the initial task).
    if let Some(current_executing_task_node) = &self.current_executing_task {
      let dependency = TaskDependency::new(task.clone(), stamper, output.clone());
      if self.store.add_task_require_dependency(current_executing_task_node, &node, dependency).is_err() {
        let current_executing_task = self.store.get_task(current_executing_task_node);
        panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which was already required", current_executing_task, task);
      }
    }

    output
  }
}

impl<T: Task> TopDownContext<T, T::Output> {
  /// Checks whether given task should be executed, returning `true` if it should be executed. A task should be executed
  /// if any of its dependencies are inconsistent, or when it has no output.
  fn should_execute_task(&mut self, node: &TaskNode) -> bool {
    // Borrow: because we pass `self` (which is `&mut self`) to `is_inconsistent` for recursive consistency checking,
    //         we need to clone and collect dependencies into a `Vec`. Otherwise we have an immutable borrow of `self`
    //         through `self.store` while we create a mutable borrow of `self`, which is not allowed.
    let dependencies: Vec<_> = self.store.get_dependencies_of_task(node).cloned().collect();
    for dependency in dependencies {
      match dependency.is_inconsistent(self) {
        Ok(Some(_)) => return true,
        Err(e) => { // Error while checking: store error and assume inconsistent
          self.dependency_check_errors.push(e);
          return true;
        }
        _ => {} // Consistent: continue checking
      }
    }
    // Task has no dependencies or all dependencies are consistent. Should only execute if it has no output, meaning
    // that it has never been executed before.
    return !self.store.task_has_output(node);
  }
}
