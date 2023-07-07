use std::io;
use std::path::PathBuf;

use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamp, OutputStamp};
use crate::Task;
use crate::tracker::Tracker;

/// A [`Tracker`] that stores [`Event`]s in a [`Vec`], useful in testing to assert that a context implementation is 
/// incremental and correct.
#[derive(Clone, Debug)]
pub struct EventTracker<T: Task> {
  events: Vec<Event<T>>,
  clear_on_build_start: bool,
}

/// Enumeration of important build events.
#[derive(Debug, Clone)]
pub enum Event<T: Task> {
  /// A require file `dependency` was created.
  RequireFile { dependency: FileDependency },
  /// A provide file `dependency` was created.
  ProvideFile { dependency: FileDependency },
  /// Start of `task` require.
  RequireTaskStart { task: T },

  /// Start of `task` execution.
  ExecuteStart { task: T },
  /// End of `task` execution, which produced `output`.
  ExecuteEnd { task: T, output: T::Output },
}

impl<T: Task> Default for EventTracker<T> {
  fn default() -> Self {
    Self { events: Vec::new(), clear_on_build_start: true }
  }
}

impl<T: Task> EventTracker<T> {
  #[inline]
  pub fn new(clear_on_build_start: bool) -> Self {
    Self {
      clear_on_build_start,
      ..Self::default()
    }
  }

  /// Returns a slice over all events.
  #[inline]
  pub fn slice(&self) -> &[Event<T>] { &self.events }
  /// Returns an iterator over all events.
  #[inline]
  pub fn iter(&self) -> impl Iterator<Item=&Event<T>> { self.events.iter() }

  /// Returns `true` if `predicate` returns `true` for any event.
  #[inline]
  pub fn any(&self, predicate: impl FnMut(&Event<T>) -> bool) -> bool { self.iter().any(predicate) }
  /// Returns the number of times `predicate` returns `true` for one event.
  #[inline]
  pub fn count(&self, predicate: impl FnMut(&&Event<T>) -> bool) -> usize { self.iter().filter(predicate).count() }
  /// Returns `true` if `predicate` returns `true` for one event.
  #[inline]
  pub fn one(&self, predicate: impl FnMut(&&Event<T>) -> bool) -> bool { self.count(predicate) == 1 }

  /// Returns `Some(index)` for the first event where `predicate` returns `true`, or `None` otherwise.
  #[inline]
  pub fn index(&self, predicate: impl FnMut(&Event<T>) -> bool) -> Option<usize> {
    self.iter().position(predicate)
  }
  /// Returns `Some(v)` for the first event where `f` returns `Some(v)`, or `None` otherwise.
  #[inline]
  pub fn find<R>(&self, f: impl FnMut(&Event<T>) -> Option<&R>) -> Option<&R> {
    self.iter().find_map(f)
  }
  /// Returns `Some((index, v))` for the first event where `f` returns `Some(v)`, or `None` otherwise.
  #[inline]
  pub fn index_find<R>(&self, mut f: impl FnMut(&Event<T>) -> Option<&R>) -> Option<(usize, &R)> {
    self.iter().enumerate().find_map(|(i, e)| f(e).map(|o| (i, o)))
  }

  /// Finds the first [`Event::RequireFile`] event that requires `path` and returns `Some(stamp)`, or `None` if no event
  /// was found.
  #[inline]
  pub fn find_require_file(&self, path: &PathBuf) -> Option<&FileStamp> {
    self.find(|e| e.match_require_file(path))
  }

  /// Returns `true` if any task was executed.
  #[inline]
  pub fn any_execute(&self) -> bool { self.any(|e| e.is_execute()) }
  /// Returns `true` if `task` was executed.
  #[inline]
  pub fn any_execute_of(&self, task: &T) -> bool { self.any(|e| e.is_execute_of(task)) }
  /// Returns `true` if `task` was executed exactly once.
  #[inline]
  pub fn one_execute_of(&self, task: &T) -> bool { self.one(|e| e.is_execute_start(task)) }

  /// Finds the first [`Event::ExecuteStart`] event for `task` and returns `Some(index)`, or `None` if no event was 
  /// found.
  #[inline]
  pub fn index_execute_start(&self, task: &T) -> Option<usize> {
    self.index(|e| e.is_execute_start(task))
  }
  /// Finds the first [`Event::ExecuteEnd`] event for `task` and returns `Some(index)`, or `None` if no event was 
  /// found.
  #[inline]
  pub fn index_execute_end(&self, task: &T) -> Option<usize> {
    self.index(|e| e.match_execute_end(task).is_some())
  }
  /// Finds the first [`Event::ExecuteEnd`] event for `task` and returns `Some((index, output))`, or `None` if no 
  /// event was found.
  #[inline]
  pub fn index_find_execute_end(&self, task: &T) -> Option<(usize, &T::Output)> {
    self.index_find(|e| e.match_execute_end(task))
  }
}

impl<T: Task> Event<T> {
  /// Returns `Some(stamp)` if this is a require file event for file at `path`, `None` otherwise.
  #[inline]
  pub fn match_require_file(&self, path: &PathBuf) -> Option<&FileStamp> {
    match self {
      Event::RequireFile { dependency: d } if d.path() == path => Some(d.stamp()),
      _ => None,
    }
  }

  /// Returns `true` if this is an execution event.
  #[inline]
  pub fn is_execute(&self) -> bool {
    match self {
      Event::ExecuteStart { .. } => true,
      Event::ExecuteEnd { .. } => true,
      _ => false,
    }
  }
  /// Returns `true` if this is an execution event of `task`.
  #[inline]
  pub fn is_execute_of(&self, task: &T) -> bool {
    match self {
      Event::ExecuteStart { task: t } if t == task => true,
      Event::ExecuteEnd { task: t, .. } if t == task => true,
      _ => false,
    }
  }
  /// Returns `true` if this is a `task` execute start event.
  #[inline]
  pub fn is_execute_start(&self, task: &T) -> bool {
    match self {
      Event::ExecuteStart { task: t } if t == task => true,
      _ => false,
    }
  }
  /// Returns `Some(output)` if this is a `task` execute end event, `None` otherwise.
  #[inline]
  pub fn match_execute_end(&self, task: &T) -> Option<&T::Output> {
    match self {
      Event::ExecuteEnd { task: t, output: o } if t == task => Some(o),
      _ => None,
    }
  }
}

impl<T: Task> Tracker<T> for EventTracker<T> {
  #[inline]
  fn require_file(&mut self, dependency: &FileDependency) {
    self.events.push(Event::RequireFile { dependency: dependency.clone() });
  }
  #[inline]
  fn provide_file(&mut self, dependency: &FileDependency) {
    self.events.push(Event::ProvideFile { dependency: dependency.clone() });
  }
  #[inline]
  fn require_task_start(&mut self, task: &T) {
    self.events.push(Event::RequireTaskStart { task: task.clone() });
  }
  #[inline]
  fn require_task_end(&mut self, _task: &T, _output: &T::Output, _was_executed: bool) {}


  #[inline]
  fn execute_task_start(&mut self, task: &T) {
    self.events.push(Event::ExecuteStart { task: task.clone() });
  }
  #[inline]
  fn execute_task_end(&mut self, task: &T, output: &T::Output) {
    self.events.push(Event::ExecuteEnd {
      task: task.clone(),
      output: output.clone()
    });
  }


  #[inline]
  fn require_top_down_initial_start(&mut self, _task: &T) {
    if self.clear_on_build_start {
      self.events.clear();
    }
  }
  #[inline]
  fn check_top_down_start(&mut self, _task: &T) {}
  #[inline]
  fn check_require_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_require_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn check_provide_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_provide_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn check_require_task_start(&mut self, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_require_task_end(&mut self, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<&OutputStamp<T::Output>>) {}
  #[inline]
  fn check_top_down_end(&mut self, _task: &T) {}
  #[inline]
  fn require_top_down_initial_end(&mut self, _task: &T, _output: &T::Output) {}


  #[inline]
  fn update_affected_by_start<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, _changed_files: I) {
    if self.clear_on_build_start {
      self.events.clear();
    }
  }
  #[inline]
  fn schedule_affected_by_file_start(&mut self, _file: &PathBuf) {}
  #[inline]
  fn check_affected_by_file_start(&mut self, _requiring_task: &T, _dependency: &FileDependency) {}
  #[inline]
  fn check_affected_by_file_end(&mut self, _requiring_task: &T, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn schedule_affected_by_file_end(&mut self, _file: &PathBuf) {}
  #[inline]
  fn schedule_affected_by_task_start(&mut self, _task: &T) {}
  #[inline]
  fn check_affected_by_required_task_start(&mut self, _requiring_task: &T, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_affected_by_required_task_end(&mut self, _requiring_task: &T, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<OutputStamp<&T::Output>>) {}
  #[inline]
  fn schedule_affected_by_task_end(&mut self, _task: &T) {}
  #[inline]
  fn schedule_task(&mut self, _task: &T) {}
  #[inline]
  fn update_affected_by_end(&mut self) {}
}
