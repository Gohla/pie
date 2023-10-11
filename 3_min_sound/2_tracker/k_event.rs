use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};
use crate::Task;
use crate::tracker::Tracker;

/// [`Tracker`] that stores [events](Event) in a [`Vec`], useful in testing to assert that a context implementation is
/// incremental and sound.
#[derive(Clone, Debug)]
pub struct EventTracker<T, O> {
  events: Vec<Event<T, O>>,
}

impl<T: Task> Default for EventTracker<T, T::Output> {
  fn default() -> Self { Self { events: Vec::new() } }
}

/// Enumeration of important build events.
#[derive(Clone, Debug)]
pub enum Event<T, O> {
  RequireFileEnd(RequireFileEnd),

  RequireTaskStart(RequireTaskStart<T>),
  RequireTaskEnd(RequireTaskEnd<T, O>),

  ExecuteStart(ExecuteStart<T>),
  ExecuteEnd(ExecuteEnd<T, O>),
}

/// End: required file at `path` using `stamper` to create `stamp`.
#[derive(Clone, Debug)]
pub struct RequireFileEnd {
  pub path: PathBuf,
  pub stamper: FileStamper,
  pub stamp: FileStamp,
  pub index: usize,
}
/// Start: require `task` using `stamper`.
#[derive(Clone, Debug)]
pub struct RequireTaskStart<T> {
  pub task: T,
  pub stamper: OutputStamper,
  pub index: usize,
}
/// End: required `task` resulting in `output`, using `stamper` to create `stamp`, and the task `was_executed`.
#[derive(Clone, Debug)]
pub struct RequireTaskEnd<T, O> {
  pub task: T,
  pub output: O,
  pub stamper: OutputStamper,
  pub stamp: OutputStamp<O>,
  pub was_executed: bool,
  pub index: usize,
}
/// Start: execute `task`.
#[derive(Clone, Debug)]
pub struct ExecuteStart<T> {
  pub task: T,
  pub index: usize,
}
/// End: executed `task`, producing `output`.
#[derive(Clone, Debug)]
pub struct ExecuteEnd<T, O> {
  pub task: T,
  pub output: O,
  pub index: usize,
}
