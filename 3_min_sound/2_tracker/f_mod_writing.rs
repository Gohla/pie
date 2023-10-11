use std::io;

use crate::dependency::{Dependency, FileDependency, Inconsistency, TaskDependency};
use crate::stamp::OutputStamper;
use crate::Task;

pub mod writing;

/// Trait for tracking build events. Can be used to implement logging, event tracing, progress tracking, metrics, etc.
#[allow(unused_variables)]
pub trait Tracker<T: Task> {
  /// Start: a new build.
  fn build_start(&mut self) {}
  /// End: completed build.
  fn build_end(&mut self) {}

  /// End: created a file `dependency`.
  fn require_file_end(&mut self, dependency: &FileDependency) {}
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
