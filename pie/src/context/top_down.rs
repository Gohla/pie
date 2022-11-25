use std::fs::File;
use std::hash::BuildHasher;
use std::path::PathBuf;

use pie_graph::Node;

use crate::{Context, Session, Task};
use crate::context::ContextShared;
use crate::dependency::Dependency;
use crate::stamp::{FileStamper, OutputStamper};
use crate::tracker::Tracker;

/// Context that incrementally executes tasks and checks dependencies recursively in a top-down manner.
#[derive(Debug)]
pub(crate) struct IncrementalTopDownContext<'p, 's, T: Task, A, H> {
  shared: ContextShared<'p, 's, T, A, H>,
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> IncrementalTopDownContext<'p, 's, T, A, H> {
  /// Creates a new [`TopDownRunner`] with given [`Tracker`].
  #[inline]
  pub(crate) fn new(session: &'s mut Session<'p, T, A, H>) -> Self {
    Self {
      shared: ContextShared::new(session),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub(crate) fn require(&mut self, task: &T) -> T::Output {
    self.shared.task_execution_stack.clear();
    self.require_task(task)
  }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> Context<T> for IncrementalTopDownContext<'p, 's, T, A, H> {
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output {
    self.shared.session.tracker.require_task(task);
    let task_node = self.shared.session.store.get_or_create_node_by_task(task.clone());
    let output = if !self.shared.session.visited.contains(&task_node) && self.should_execute_task(task_node) { // Execute the task, cache and return up-to-date output.
      self.shared.session.store.reset_task(&task_node);
      
      if let Some(current_task_node) = self.shared.task_execution_stack.last() {
        if let Err(pie_graph::Error::CycleDetected) = self.shared.session.store.add_task_dependency_edge(*current_task_node, task_node) {
          let current_task = self.shared.session.store.task_by_node(current_task_node);
          let task_stack: Vec<_> = self.shared.task_execution_stack.iter().map(|task_node| self.shared.session.store.task_by_node(task_node)).collect();
          panic!("Cyclic task dependency; current task '{:?}' is requiring task '{:?}' which was already required. Task stack: {:?}", current_task, task, task_stack);
        }
      }
      
      self.shared.pre_execute(task, task_node);
      let output = task.execute(self);
      self.shared.post_execute(task, task_node, &output);
      
      if let Some(current_task_node) = self.shared.task_execution_stack.last() {
        self.shared.session.store.add_to_dependencies_of_task(*current_task_node, Dependency::require_task(task.clone(), output.clone(), stamper));
      }
      output
    } else { // Return already up-to-date output.
      // Unwrap OK: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.shared.session.store.get_task_output(task_node).unwrap().clone();
      output
    };
    self.shared.session.visited.insert(task_node);
    output
  }

  #[inline]
  fn require_file_with_stamper(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<File, std::io::Error> {
    self.shared.require_file_with_stamper(path, stamper)
  }
  #[inline]
  fn provide_file_with_stamper(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<(), std::io::Error> {
    self.shared.provide_file_with_stamper(path, stamper)
  }

  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { self.shared.default_output_stamper() }
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { self.shared.default_require_file_stamper() }
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { self.shared.default_provide_file_stamper() }
}

impl<'p, 's, T: Task, A: Tracker<T>, H: BuildHasher + Default> IncrementalTopDownContext<'p, 's, T, A, H> {
  fn should_execute_task(&mut self, task_node: Node) -> bool {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.shared.session.store.remove_dependencies_of_task(&task_node);
    if let Some(task_dependencies) = task_dependencies {
      // Task has been executed before, check whether all its dependencies are still consistent. If one or more are not,
      // we need to execute the task.
      for task_dependency in &task_dependencies {
        match task_dependency.is_consistent(self) {
          Ok(false) => return true, // Not consistent -> should execute task.
          Err(e) => { // Error -> store error and assume not consistent -> should execute task.
            self.shared.session.dependency_check_errors.push(e);
            return true;
          }
          _ => {} // Continue to check other dependencies.
        }
      }
      // Task is consistent and does not need to be executed. Restore the previous dependencies.
      self.shared.session.store.set_dependencies_of_task(task_node, task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      false
    } else if self.shared.session.store.task_has_output(task_node) {
      // Task has no dependencies; but has been executed before, so it never has to be executed again.
      false
    } else {
      // Task has not been executed, so we need to execute it.
      true
    }
  }
}
