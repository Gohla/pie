use std::path::Path;

use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};
use crate::Task;

pub mod writing;
pub mod event;

/// Trait for tracking build events. Can be used to implement logging, event tracing, progress tracking, metrics, etc.
#[allow(unused_variables)]
pub trait Tracker<T: Task> {
  /// Start a new build.
  fn build_start(&mut self) {}
  /// A build has been completed.
  fn build_end(&mut self) {}

  /// A file at `path` has been required, using `stamper` to create `stamp`.
  fn required_file(&mut self, path: &Path, stamper: &FileStamper, stamp: &FileStamp) {}
  /// Require `task` using `stamper`.
  fn require_task(&mut self, task: &T, stamper: &OutputStamper) {}
  /// A `task` has been required, resulting in consistent `output`, using `stamper` to create `stamp`, and task 
  /// `was_executed`.
  fn required_task(&mut self, task: &T, output: &T::Output, stamper: &OutputStamper, stamp: &OutputStamp<T::Output>, was_executed: bool) {}

  /// Execute `task`.
  fn execute(&mut self, task: &T) {}
  /// A `task` has been executed, producing `output`.
  fn executed(&mut self, task: &T, output: &T::Output) {}
}
