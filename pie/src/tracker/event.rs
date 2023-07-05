use std::io;
use std::path::PathBuf;

use crate::dependency::{FileDependency, TaskDependency};
use crate::stamp::{FileStamp, OutputStamp};
use crate::Task;
use crate::tracker::Tracker;

/// A [`Tracker`] that stores [`Event`]s in a [`Vec`], useful in testing situations where we check build events after
/// building. 
#[derive(Clone, Debug)]
pub struct EventTracker<T: Task> {
  events: Vec<Event<T>>,
  clear_on_build_start: bool,
}

#[derive(Debug, Clone)]
pub enum Event<T: Task> {
  RequireFile(FileDependency),
  ProvideFile(FileDependency),
  RequireTask(T),

  ExecuteTaskStart(T),
  ExecuteTaskEnd(T, T::Output),
}

impl<T: Task> Event<T> {
  #[inline]
  pub fn match_require_file(&self, path: &PathBuf) -> Option<&FileStamp> {
    match self {
      Event::RequireFile(d) if d.path() == path => Some(d.stamp()),
      _ => None,
    }
  }

  #[inline]
  pub fn is_execute(&self) -> bool {
    match self {
      Event::ExecuteTaskStart(_) => true,
      Event::ExecuteTaskEnd(_, _) => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_execute_of(&self, task: &T) -> bool {
    match self {
      Event::ExecuteTaskStart(t) if t == task => true,
      Event::ExecuteTaskEnd(t, _) if t == task => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_execute_start_of(&self, task: &T) -> bool {
    match self {
      Event::ExecuteTaskStart(t) if t == task => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_execute_end_of(&self, task: &T) -> bool {
    match self {
      Event::ExecuteTaskEnd(t, _) if t == task => true,
      _ => false,
    }
  }
  #[inline]
  pub fn match_execute_end(&self, task: &T) -> Option<&T::Output> {
    match self {
      Event::ExecuteTaskEnd(t, o) if t == task => Some(o),
      _ => None,
    }
  }
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
  /// Returns the event at given `index`, or `None` if `index` is out of bounds.
  #[inline]
  pub fn get(&self, index: usize) -> Option<&Event<T>> {
    self.events.get(index)
  }
  /// Returns the event at given `offset` from the end, or `None` if `offset` is out of bounds.
  #[inline]
  pub fn get_from_end(&self, offset: usize) -> Option<&Event<T>> {
    self.get(self.events.len() - 1 - offset)
  }

  /// Returns `Some(v)` for the first event where `f` returns `Some(v)`, or `None` otherwise.
  #[inline]
  pub fn find<R>(&self, f: impl FnMut(&Event<T>) -> Option<&R>) -> Option<&R> { self.iter().find_map(f) }


  #[inline]
  pub fn find_require_file(&self, path: &PathBuf) -> Option<&FileStamp> {
    self.find(|e| e.match_require_file(path))
  }


  #[inline]
  pub fn any_execute(&self) -> bool { self.any(|e| e.is_execute()) }
  #[inline]
  pub fn any_execute_of(&self, task: &T) -> bool { self.any(|e| e.is_execute_of(task)) }
  #[inline]
  pub fn index_execute_start(&self, task: &T) -> Option<usize> {
    self.index(|e| e.is_execute_start_of(task))
  }
  #[inline]
  pub fn index_execute_end(&self, task: &T) -> Option<usize> {
    self.index(|e| e.is_execute_end_of(task))
  }
  #[inline]
  pub fn find_execute_end(&self, task: &T) -> Option<&T::Output> {
    self.find(|e| e.match_execute_end(task))
  }


  #[inline]
  pub fn take(&mut self) -> Vec<Event<T>> { std::mem::take(&mut self.events) }
  #[inline]
  pub fn clear(&mut self) { self.events.clear(); }
}

impl<T: Task> Tracker<T> for EventTracker<T> {
  #[inline]
  fn require_file(&mut self, dependency: &FileDependency) {
    self.events.push(Event::RequireFile(dependency.clone()));
  }
  #[inline]
  fn provide_file(&mut self, dependency: &FileDependency) {
    self.events.push(Event::ProvideFile(dependency.clone()));
  }
  #[inline]
  fn require_task(&mut self, task: &T) {
    self.events.push(Event::RequireTask(task.clone()));
  }

  #[inline]
  fn execute_task_start(&mut self, task: &T) {
    self.events.push(Event::ExecuteTaskStart(task.clone()));
  }
  #[inline]
  fn execute_task_end(&mut self, task: &T, output: &T::Output) {
    self.events.push(Event::ExecuteTaskEnd(task.clone(), output.clone()));
  }
  #[inline]
  fn up_to_date(&mut self, _task: &T) {}

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
  fn update_affected_by_end(&mut self) {}
  #[inline]
  fn schedule_affected_by_file_start(&mut self, _file: &PathBuf) {}
  #[inline]
  fn check_affected_by_file_end(&mut self, _requiring_task: &T, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &io::Error>) {}
  #[inline]
  fn check_affected_by_file_start(&mut self, _requiring_task: &T, _dependency: &FileDependency) {}
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
}
