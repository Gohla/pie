use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::path::PathBuf;

use pie_graph::Node;

use crate::{Context, Task};
use crate::dependency::{FileDependency, TaskDependency};
use crate::prelude::DynOutput;
use crate::runner::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};

/// Incremental runner that checks dependencies recursively in a top-down manner.
pub struct TopDownRunner<R: Tracker> {
  store: Store<Self>,
  tracker: R,

  visited: HashSet<TaskNode>,
  task_execution_stack: Vec<TaskNode>,
  dependency_check_errors: Vec<Box<dyn Error>>,
}

impl Default for TopDownRunner<NoopTracker> {
  fn default() -> Self { Self::with_tracker(NoopTracker) }
}

impl TopDownRunner<NoopTracker> {
  /// Creates a new `[TopDownRunner]` without a `[Tracker]`.
  #[inline]
  pub fn new() -> Self { Default::default() }
}

impl<R: Tracker> TopDownRunner<R> {
  /// Creates a new `[TopDownRunner]` with given `[Tracker]`.
  #[inline]
  pub fn with_tracker(tracker: R) -> Self {
    Self {
      store: Default::default(),
      tracker,

      visited: Default::default(),
      task_execution_stack: Default::default(),
      dependency_check_errors: Default::default(),
    }
  }

  /// Requires given `[task]`, returning its up-to-date output, or an error indicating failure to check consistency of 
  /// one or more dependencies.
  #[inline]
  pub fn require_initial<T: Task>(&mut self, task: &T) -> Result<T::Output, (T::Output, &[Box<dyn Error>])> {
    self.visited.clear();
    self.task_execution_stack.clear();
    self.dependency_check_errors.clear();
    let output = self.require_task::<T>(task);
    if self.dependency_check_errors.is_empty() {
      Ok(output)
    } else {
      Err((output, &self.dependency_check_errors))
    }
  }
  #[inline]
  pub fn tracker(&self) -> &R { &self.tracker }
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut R { &mut self.tracker }
}

impl<R: Tracker> Context for TopDownRunner<R> {
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output {
    let dyn_task = task.as_dyn();
    self.tracker.require_task(dyn_task);
    let task_node = self.store.get_or_create_node_by_task(task.clone_box_dyn());
    if !self.visited.contains(&task_node) && self.should_execute_task(task_node) { // Execute the task, cache and return up-to-date output.
      self.store.reset_task(&task_node);
      // Check for cyclic dependency
      if let Some(current_task_node) = self.task_execution_stack.last() {
        if let Err(pie_graph::Error::CycleDetected) = self.store.add_task_dependency_edge(*current_task_node, task_node) {
          let current_task = self.store.task_by_node(current_task_node);
          let task_stack: Vec<_> = self.task_execution_stack.iter().map(|task_node| self.store.task_by_node(task_node)).collect();
          panic!("Cyclic task dependency; current task '{:?}' is requiring task '{:?}' which was already required. Task stack: {:?}", current_task, task, task_stack);
        }
      }
      // Execute task
      self.task_execution_stack.push(task_node);
      self.tracker.execute_task_start(dyn_task);
      let output = task.execute(self);
      self.tracker.execute_task_end(dyn_task, &output as &dyn DynOutput);
      self.task_execution_stack.pop();
      // Store dependency and output.
      if let Some(current_task_node) = self.task_execution_stack.last() {
        self.store.add_to_dependencies_of_task(*current_task_node, Box::new(TaskDependency::new(task.clone(), output.clone())));
      }
      self.store.set_task_output::<T>(task_node, output.clone());
      self.visited.insert(task_node);
      output
    } else { // Return already up-to-date output.
      // Unwrap OK: if we should not execute the task, it must have been executed before, and therefore it has an output.
      let output = self.store.get_task_output::<T>(task_node).unwrap().clone();
      output
    }
  }

  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> {
    self.tracker.require_file(path);
    let file_node = self.store.get_or_create_file_node(path);
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    let opened = dependency.open();
    if let Some(current_requiring_task_node) = self.task_execution_stack.last() {
      if let Some(providing_task_node) = self.store.get_providing_task_node(&file_node) {
        if !self.store.contains_transitive_task_dependency(current_requiring_task_node, &providing_task_node) {
          let current_requiring_task = self.store.task_by_node(current_requiring_task_node);
          let providing_task = self.store.task_by_node(&providing_task_node);
          panic!("Hidden dependency; file '{}' is required by the current task '{:?}' without a dependency to providing task: {:?}", path.display(), current_requiring_task, providing_task);
        }
      }
      self.store.add_file_require_dependency(*current_requiring_task_node, file_node, dependency);
    }
    opened
  }

  fn provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error> {
    self.tracker.provide_file(path);
    let file_node = self.store.get_or_create_file_node(path);
    let dependency = FileDependency::new(path.clone()).map_err(|e| e.kind())?;
    if let Some(current_providing_task_node) = self.task_execution_stack.last() {
      if let Some(previous_providing_task_node) = self.store.get_providing_task_node(&file_node) {
        let current_providing_task = self.store.task_by_node(current_providing_task_node);
        let previous_providing_task = self.store.task_by_node(&previous_providing_task_node);
        panic!("Overlapping provided file; file '{}' is provided by the current task '{:?}' that was previously provided by task: {:?}", path.display(), current_providing_task, previous_providing_task);
      }
      for requiring_task_node in self.store.get_requiring_task_nodes(&file_node) {
        if !self.store.contains_transitive_task_dependency(&requiring_task_node, current_providing_task_node) {
          let current_providing_task = self.store.task_by_node(current_providing_task_node);
          let requiring_task = self.store.task_by_node(&requiring_task_node);
          panic!("Hidden dependency; file '{}' is provided by the current task '{:?}' without a dependency from requiring task '{:?}' to the current providing task", path.display(), current_providing_task, requiring_task);
        }
      }
      self.store.add_file_provide_dependency(*current_providing_task_node, file_node, dependency);
    }
    Ok(())
  }
}

impl<R: Tracker> TopDownRunner<R> {
  fn should_execute_task(&mut self, task_node: Node) -> bool {
    // Remove task dependencies so that we get ownership over the list of dependencies. If the task does not need to be
    // executed, we will restore the dependencies again.
    let task_dependencies = self.store.remove_dependencies_of_task(&task_node);
    if let Some(task_dependencies) = task_dependencies {
      // Task has been executed before, check whether all its dependencies are still consistent. If one or more are not,
      // we need to execute the task.
      for task_dependency in &task_dependencies {
        match task_dependency.is_consistent(self) {
          Ok(false) => return true, // Not consistent -> should execute task.
          Err(e) => { // Error -> store error and assume not consistent -> should execute task.
            self.dependency_check_errors.push(e);
            return true;
          }
          _ => {} // Continue to check other dependencies.
        }
      }
      // Task is consistent and does not need to be executed. Restore the previous dependencies.
      self.store.set_dependencies_of_task(task_node, task_dependencies); // OPTO: removing and inserting into a HashMap due to ownership requirements.
      false
    } else if self.store.task_has_output(task_node) {
      // Task has no dependencies; but has been executed before, so it never has to be executed again.
      false
    } else {
      // Task has not been executed, so we need to execute it.
      true
    }
  }
}
