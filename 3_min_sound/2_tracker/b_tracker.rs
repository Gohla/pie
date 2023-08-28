use crate::dependency::FileDependency;
use crate::Task;

/// Trait for tracking build events. Can be used to implement logging, event tracing, progress tracking, metrics, etc.
#[allow(unused_variables)]
pub trait Tracker<T: Task> {
  // Dependencies
  fn required_file(&mut self, dependency: &FileDependency) {}
  fn required_task(&mut self, task: &T, output: &T::Output) {}
  // Task execution
  fn execute_task_start(&mut self, task: &T) {}
  fn execute_task_end(&mut self, task: &T, output: &T::Output) {}
  // Top-down builds
  fn require_top_down_initial_start(&mut self, task: &T) {}
  fn require_top_down_initial_end(&mut self, task: &T, output: &T::Output) {}
}
