use std::fs::File;
use std::io;
use std::path::Path;

use crate::{Context, Task};
use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::stamp::{FileStamper, OutputStamper};
use crate::store::{Store, TaskNode};

pub struct TopDownContext<T, O> {
  store: Store<T, O>,
  current_executing_task: Option<TaskNode>,
}

impl<T: Task> TopDownContext<T, T::Output> {
  pub fn new(store: Store<T, T::Output>) -> Self {
    Self {
      store,
      current_executing_task: None,
    }
  }

  pub fn require(&mut self, task: &T) -> T::Output {
    self.current_executing_task = None;
    let output = self.require_task(task);
    output
  }
}

impl<T: Task> Context<T> for TopDownContext<T, T::Output> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    let node = self.store.get_or_create_task_node(task);

    self.reserve_task_require_dependency(&node, task);

    let output = if self.should_execute_task(&node, task) { // Execute the task, cache and return up-to-date output.
      self.store.reset_task(&node);
      let previous_executing_task = self.current_executing_task.replace(node);
      let output = task.execute(self);
      self.current_executing_task = previous_executing_task;
      self.store.set_task_output(&node, output.clone());
      output
    } else { // Return already up-to-date output.
      // No panic: if we should not execute the task, it must have been executed before, and therefore it has an output.
      self.store.get_task_output(&node).clone()
    };

    let dependency = TaskDependency::new(task.clone(), stamper, output.clone());
    self.update_reserved_task_require_dependency(&node, dependency);

    output
  }

  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error> {
    let path = path.as_ref();
    let (dependency, file) = FileDependency::new(path, stamper)?;
    let node = self.store.get_or_create_file_node(path);
    let Some(current_executing_task_node) = &self.current_executing_task else {
      panic!("BUG: requiring file without a current executing task");
    };
    self.store.add_file_require_dependency(current_executing_task_node, &node, dependency);
    Ok(file)
  }
}

impl<T: Task> TopDownContext<T, T::Output> {
  /// Reserve a task require dependency from the current executing task to `dst`, detecting cycles before we execute, 
  /// preventing infinite recursion/loops.
  fn reserve_task_require_dependency(&mut self, dst: &TaskNode, dst_task: &T) {
    let Some(src) = &self.current_executing_task else {
      return; // No task is executing (i.e., `dst` is the initial required task), so there is no dependency to reserve.
    };
    if let Err(pie_graph::Error::CycleDetected) = self.store.reserve_task_require_dependency(src, dst) {
      let src_task = self.session.store.get_task(src);
      panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which was already required", src_task, dst_task);
    }
  }

  /// Update the reserved task dependency from the current executing task to `dst` with an actual task dependency.
  fn update_reserved_task_require_dependency(&mut self, dst: &TaskNode, dependency: TaskDependency<T, T::Output>) {
    let Some(src) = &self.current_executing_task else {
      return; // No task is executing (i.e., `dst` is the initial required task), so there is no dependency to update.
    };
    self.store.update_reserved_task_require_dependency(src, dst, dependency);
  }

  /// Checks whether the task should be executed, returning `true` if it should be executed.
  fn should_execute_task(&mut self, node: &TaskNode, task: &T) -> bool {
    let mut has_dependencies = false;
    let mut is_dependency_inconsistent = false;
    // Borrow: because we pass `&mut self` to `is_dependency_inconsistent` for recursive consistency checking, we need
    //         to clone and collect dependencies into a `Vec` here.
    let dependencies: Vec<_> = self.shared.session.store.get_dependencies_of_task(node).cloned().collect();
    for dependency in dependencies {
      has_dependencies = true;
      is_dependency_inconsistent |= self.is_dependency_inconsistent(dependency);
    }

    match has_dependencies {
      true => is_dependency_inconsistent, // If the task has dependencies, execute if any dependency is inconsistent.
      false => !self.shared.session.store.task_has_output(node), // If task has no dependencies, execute if it has no output, meaning that it has never been executed.
    }
  }

  #[allow(clippy::wrong_self_convention)]
  #[inline]
  fn is_dependency_inconsistent(&mut self, dependency: Option<Dependency<T, T::Output>>) -> bool {
    let Some(dependency) = dependency else {
      panic!("BUG: checking reserved dependency for inconsistency");
    };
    let inconsistent = dependency.is_inconsistent(self);
    match inconsistent {
      Ok(Some(_)) => true,
      Err(e) => { // Error while checking: store error and assume inconsistent
        self.dependency_check_errors.push(e);
        true
      }
      _ => false,
    }
  }
}
