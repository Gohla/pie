use std::io;

use crate::dependency::{Dependency, FileDependency, Inconsistency, TaskDependency};
use crate::stamp::OutputStamper;
use crate::Task;

pub mod writing;
pub mod event;

/// Trait for tracking build events. Can be used to implement logging, event tracing, progress tracking, metrics, etc.
#[allow(unused_variables)]
pub trait Tracker<T: Task> {
  /// Start: a new build.
  fn build_start(&mut self) {}
  /// End: completed build.
  fn build_end(&mut self) {}

  /// End: created a require file `dependency`.
  fn require_file_end(&mut self, dependency: &FileDependency) {}
  /// End: created a provide file `dependency`.
  fn provide_file_end(&mut self, dependency: &FileDependency) {}
  /// Start: require `task` using `stamper`.
  fn require_task_start(&mut self, task: &T, stamper: &OutputStamper) {}
  /// End: required a task, resulting in a task `dependency` and `output`, and the task `was_executed`.
  fn require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, output: &T::Output, was_executed: bool) {}

  /// Start: check consistency of `dependency`.
  fn check_dependency_start(&mut self, dependency: &Dependency<T, T::Output>) {}
  /// End: checked consistency of `dependency`, possibly found `inconsistency`.
  fn check_dependency_end(
    &mut self,
    dependency: &Dependency<T, T::Output>,
    inconsistency: Result<Option<&Inconsistency<T::Output>>, &io::Error>
  ) {}

  /// Start: execute `task`.
  fn execute_start(&mut self, task: &T) {}
  /// End: executed `task` resulting in `output`.
  fn execute_end(&mut self, task: &T, output: &T::Output) {}
}

/// [`Tracker`] that does nothing.
#[derive(Copy, Clone, Debug)]
pub struct NoopTracker;
impl<T: Task> Tracker<T> for NoopTracker {}

/// [`Tracker`] that forwards build events to 2 trackers.
#[derive(Copy, Clone, Debug)]
pub struct CompositeTracker<A1, A2>(pub A1, pub A2);
impl<T: Task, A1: Tracker<T>, A2: Tracker<T>> Tracker<T> for CompositeTracker<A1, A2> {
  fn build_start(&mut self) {
    self.0.build_start();
    self.1.build_start();
  }
  fn build_end(&mut self) {
    self.0.build_end();
    self.1.build_end();
  }

  fn provide_file_end(&mut self, dependency: &FileDependency) {
    self.0.provide_file_end(dependency);
    self.1.provide_file_end(dependency);
  }
  fn require_file_end(&mut self, dependency: &FileDependency) {
    self.0.require_file_end(dependency);
    self.1.require_file_end(dependency);
  }
  fn require_task_start(&mut self, task: &T, stamper: &OutputStamper) {
    self.0.require_task_start(task, stamper);
    self.1.require_task_start(task, stamper);
  }
  fn require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, output: &T::Output, was_executed: bool) {
    self.0.require_task_end(dependency, output, was_executed);
    self.1.require_task_end(dependency, output, was_executed);
  }

  fn check_dependency_start(&mut self, dependency: &Dependency<T, T::Output>) {
    self.0.check_dependency_start(dependency);
    self.1.check_dependency_start(dependency);
  }
  fn check_dependency_end(
    &mut self,
    dependency: &Dependency<T, T::Output>,
    inconsistency: Result<Option<&Inconsistency<T::Output>>, &io::Error>
  ) {
    self.0.check_dependency_end(dependency, inconsistency);
    self.1.check_dependency_end(dependency, inconsistency);
  }

  fn execute_start(&mut self, task: &T) {
    self.0.execute_start(task);
    self.1.execute_start(task);
  }
  fn execute_end(&mut self, task: &T, output: &T::Output) {
    self.0.execute_end(task, output);
    self.1.execute_end(task, output);
  }
}
