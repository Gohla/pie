use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, fs, Session, Task};
use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::TaskNode;

pub struct TopDownContext<'p, 's, T, O> {
  session: &'s mut Session<'p, T, O>,
}

impl<'p, 's, T: Task> TopDownContext<'p, 's, T, T::Output> {
  pub fn new(session: &'s mut Session<'p, T, T::Output>) -> Self { Self { session } }

  pub fn require_initial(&mut self, task: &T) -> T::Output {
    self.require_task(task)
  }
}

impl<'p, 's, T: Task> Context<T> for TopDownContext<'p, 's, T, T::Output> {
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    let Some(current_executing_task_node) = &self.session.current_executing_task else {
      return fs::open_if_file(path); // No current executing task, so no dependency needs to be made.
    };
    let path = path.as_ref();
    let node = self.session.store.get_or_create_file_node(path);
    let (dependency, file) = FileDependency::new_with_file(path, stamper)?;
    self.session.store.add_file_require_dependency(current_executing_task_node, &node, dependency);
    Ok(file)
  }

  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    let node = self.session.store.get_or_create_task_node(task);

    // Get required task output by executing it if needed, or by getting the output from the store if not needed.
    let already_consistent = self.session.consistent.contains(&node);
    let output = if !already_consistent && self.should_execute_task(&node) {
      self.session.store.reset_task(&node);
      let previous_executing_task = self.session.current_executing_task.replace(node);
      let output = task.execute(self);
      self.session.current_executing_task = previous_executing_task;
      self.session.store.set_task_output(&node, output.clone());
      output
    } else {
      // Correctness: when `should_execute_task` returns `true`, the above block is executed. Otherwise this block is 
      // executed and `should_execute_task` ensures that the task has an output.
      self.session.store.get_task_output(&node).clone()
    };

    // Create task require dependency if a task is currently executing (i.e., we are not requiring the initial task).
    if let Some(current_executing_task_node) = &self.session.current_executing_task {
      let dependency = TaskDependency::new(task.clone(), stamper, output.clone());
      if self.session.store.add_task_require_dependency(current_executing_task_node, &node, dependency).is_err() {
        let current_executing_task = self.session.store.get_task(current_executing_task_node);
        panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which was already required", current_executing_task, task);
      }
    }

    self.session.consistent.insert(node);
    output
  }
}

impl<'p, 's, T: Task> TopDownContext<'p, 's, T, T::Output> {
  /// Checks whether given task should be executed, returning `true` if it should be executed. A task should be executed
  /// if any of its dependencies are inconsistent, or when it has no output.
  fn should_execute_task(&mut self, node: &TaskNode) -> bool {
    // Borrow: because we pass `self` (which is `&mut self`) to `is_inconsistent` for recursive consistency checking, 
    //         we need to clone and collect dependencies into a `Vec`. Otherwise we have an immutable borrow of `self`
    //         through `self.store` while we create a mutable borrow of `self`, which is not allowed.
    let dependencies: Vec<_> = self.session.store.get_dependencies_of_task(node).cloned().collect();
    for dependency in dependencies {
      match dependency.is_inconsistent(self) {
        Ok(Some(_)) => return true,
        Err(e) => { // Error while checking: store error and assume inconsistent
          self.session.dependency_check_errors.push(e);
          return true;
        }
        _ => {} // Consistent: continue checking
      }
    }
    // Task has no dependencies or all dependencies are consistent. Should only execute if it has no output, meaning
    // that it has never been executed before.
    return !self.session.store.task_has_output(node);
  }
}
