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
  ProvideFileEnd(FileDependencyEnd),
  RequireFileEnd(FileDependencyEnd),

  RequireTaskStart(RequireTaskStart<T>),
  RequireTaskEnd(RequireTaskEnd<T, O>),

  ExecuteStart(ExecuteStart<T>),
  ExecuteEnd(ExecuteEnd<T, O>),
}

/// End: required/provided file at `path` using `stamper` to create `stamp`.
#[derive(Clone, Debug)]
pub struct FileDependencyEnd {
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
  pub stamper: OutputStamper,
  pub stamp: OutputStamp<O>,
  pub output: O,
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

impl<T: Task> Tracker<T> for EventTracker<T, T::Output> {
  fn build_start(&mut self) {
    self.events.clear();
  }

  fn require_file_end(&mut self, dependency: &FileDependency) {
    let data = FileDependencyEnd {
      path: dependency.path().into(),
      stamper: *dependency.stamper(),
      stamp: *dependency.stamp(),
      index: self.events.len()
    };
    self.events.push(Event::RequireFileEnd(data));
  }
  fn provide_file_end(&mut self, dependency: &FileDependency) {
    let data = FileDependencyEnd {
      path: dependency.path().into(),
      stamper: *dependency.stamper(),
      stamp: *dependency.stamp(),
      index: self.events.len()
    };
    self.events.push(Event::ProvideFileEnd(data));
  }
  fn require_task_start(&mut self, task: &T, stamper: &OutputStamper) {
    let data = RequireTaskStart { task: task.clone(), stamper: stamper.clone(), index: self.events.len() };
    self.events.push(Event::RequireTaskStart(data));
  }
  fn require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, output: &T::Output, was_executed: bool) {
    let data = RequireTaskEnd {
      task: dependency.task().clone(),
      stamper: *dependency.stamper(),
      stamp: dependency.stamp().clone(),
      output: output.clone(),
      was_executed,
      index: self.events.len()
    };
    self.events.push(Event::RequireTaskEnd(data));
  }

  fn execute_start(&mut self, task: &T) {
    let data = ExecuteStart { task: task.clone(), index: self.events.len() };
    self.events.push(Event::ExecuteStart(data));
  }
  fn execute_end(&mut self, task: &T, output: &T::Output) {
    let data = ExecuteEnd { task: task.clone(), output: output.clone(), index: self.events.len() };
    self.events.push(Event::ExecuteEnd(data));
  }
}

impl<T: Task> Event<T, T::Output> {
  /// Returns `Some(&data)` if this is a [require file end event](Event::RequireFileEnd) for file at `path`, or `None`
  /// otherwise.
  pub fn match_require_file_end(&self, path: impl AsRef<Path>) -> Option<&FileDependencyEnd> {
    let path = path.as_ref();
    match self {
      Event::RequireFileEnd(data) if data.path == path => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [provide file end event](Event::ProvideFileEnd) for file at `path`, or `None`
  /// otherwise.
  pub fn match_provide_file_end(&self, path: impl AsRef<Path>) -> Option<&FileDependencyEnd> {
    let path = path.as_ref();
    match self {
      Event::ProvideFileEnd(data) if data.path == path => Some(data),
      _ => None,
    }
  }

  /// Returns `Some(&data)` if this is a [require task start event](Event::RequireTaskStart) for `task`, or `None`
  /// otherwise.
  pub fn match_require_task_start(&self, task: &T) -> Option<&RequireTaskStart<T>> {
    match self {
      Event::RequireTaskStart(data) if data.task == *task => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [require task start event](Event::RequireTaskStart) for `task`, or `None`
  /// otherwise.
  pub fn match_require_task_end(&self, task: &T) -> Option<&RequireTaskEnd<T, T::Output>> {
    match self {
      Event::RequireTaskEnd(data) if data.task == *task => Some(data),
      _ => None,
    }
  }

  /// Returns `true` if this is a task execute [start](Event::ExecuteStart) or [end](Event::ExecuteEnd) event.
  pub fn is_execute(&self) -> bool {
    match self {
      Event::ExecuteStart(_) | Event::ExecuteEnd(_) => true,
      _ => false,
    }
  }
  /// Returns `true` if this is an execute [start](Event::ExecuteStart) or [end](Event::ExecuteEnd) event for `task`.
  pub fn is_execute_of(&self, task: &T) -> bool {
    match self {
      Event::ExecuteStart(ExecuteStart { task: t, .. }) |
      Event::ExecuteEnd(ExecuteEnd { task: t, .. }) if t == task => true,
      _ => false,
    }
  }
  /// Returns `Some(&data)` if this is a [task execute start event](Event::ExecuteStart) for `task`, or `None`
  /// otherwise.
  pub fn match_execute_start(&self, task: &T) -> Option<&ExecuteStart<T>> {
    match self {
      Event::ExecuteStart(data) if data.task == *task => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [task execute end event](Event::ExecuteStart) for `task`, or `None` otherwise.
  pub fn match_execute_end(&self, task: &T) -> Option<&ExecuteEnd<T, T::Output>> {
    match self {
      Event::ExecuteEnd(data) if data.task == *task => Some(data),
      _ => None,
    }
  }
}

impl<T: Task> EventTracker<T, T::Output> {
  /// Returns a slice over all events.
  pub fn slice(&self) -> &[Event<T, T::Output>] {
    &self.events
  }
  /// Returns an iterator over all events.
  pub fn iter(&self) -> impl Iterator<Item=&Event<T, T::Output>> {
    self.events.iter()
  }

  /// Returns `true` if `predicate` returns `true` for any event.
  pub fn any(&self, predicate: impl FnMut(&Event<T, T::Output>) -> bool) -> bool {
    self.iter().any(predicate)
  }
  /// Returns `true` if `predicate` returns `true` for exactly one event.
  pub fn one(&self, predicate: impl FnMut(&&Event<T, T::Output>) -> bool) -> bool {
    self.iter().filter(predicate).count() == 1
  }

  /// Returns `Some(v)` for the first event `e` where `f(e)` returns `Some(v)`, or `None` otherwise.
  pub fn find_map<R>(&self, f: impl FnMut(&Event<T, T::Output>) -> Option<&R>) -> Option<&R> {
    self.iter().find_map(f)
  }


  /// Finds the first [require file end event](Event::RequireFileEnd) for `path` and returns `Some(&data)`, or `None`
  /// otherwise.
  pub fn first_require_file(&self, path: &PathBuf) -> Option<&FileDependencyEnd> {
    self.find_map(|e| e.match_require_file_end(path))
  }
  /// Finds the first [require file end event](Event::RequireFileEnd) for `path` and returns `Some(&index)`, or `None`
  /// otherwise.
  pub fn first_require_file_index(&self, path: &PathBuf) -> Option<&usize> {
    self.first_require_file(path).map(|d| &d.index)
  }
  /// Finds the first [provide file end event](Event::ProvideFileEnd) for `path` and returns `Some(&data)`, or `None`
  /// otherwise.
  pub fn first_provide_file(&self, path: &PathBuf) -> Option<&FileDependencyEnd> {
    self.find_map(|e| e.match_provide_file_end(path))
  }
  /// Finds the first [provide file end event](Event::ProvideFileEnd) for `path` and returns `Some(&index)`, or `None`
  /// otherwise.
  pub fn first_provide_file_index(&self, path: &PathBuf) -> Option<&usize> {
    self.first_provide_file(path).map(|d| &d.index)
  }

  /// Finds the first require [start](Event::RequireTaskStart) and [end](Event::RequireTaskEnd) event for `task` and
  /// returns `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_require_task(&self, task: &T) -> Option<(&RequireTaskStart<T>, &RequireTaskEnd<T, T::Output>)> {
    let start_data = self.find_map(|e| e.match_require_task_start(task));
    let end_data = self.find_map(|e| e.match_require_task_end(task));
    start_data.zip(end_data)
  }
  /// Finds the first require [start](Event::RequireTaskStart) and [end](Event::RequireTaskEnd) event for `task` and
  /// returns `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_require_task_range(&self, task: &T) -> Option<RangeInclusive<usize>> {
    self.first_require_task(task).map(|(s, e)| s.index..=e.index)
  }

  /// Returns `true` if any task was executed.
  pub fn any_execute(&self) -> bool {
    self.any(|e| e.is_execute())
  }
  /// Returns `true` if `task` was executed.
  pub fn any_execute_of(&self, task: &T) -> bool {
    self.any(|e| e.is_execute_of(task))
  }
  /// Returns `true` if `task` was executed exactly once.
  pub fn one_execute_of(&self, task: &T) -> bool {
    self.one(|e| e.match_execute_start(task).is_some())
  }

  /// Finds the first execute [start](Event::ExecuteStart) and [end](Event::ExecuteEnd) event for `task` and returns
  /// `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_execute(&self, task: &T) -> Option<(&ExecuteStart<T>, &ExecuteEnd<T, T::Output>)> {
    let start_data = self.find_map(|e| e.match_execute_start(task));
    let end_data = self.find_map(|e| e.match_execute_end(task));
    start_data.zip(end_data)
  }
  /// Finds the first execute [start](Event::ExecuteStart) and [end](Event::ExecuteEnd) event for `task` and returns
  /// `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_execute_range(&self, task: &T) -> Option<RangeInclusive<usize>> {
    self.first_execute(task).map(|(s, e)| s.index..=e.index)
  }
}
