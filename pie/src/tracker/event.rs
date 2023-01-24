use std::error::Error;
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
}

#[derive(Debug, Clone)]
pub enum Event<T: Task> {
  RequireFile(PathBuf),
  ProvideFile(PathBuf),
  RequireTask(T),

  ExecuteTaskStart(T),
  ExecuteTaskEnd(T, T::Output),
}

impl<T: Task> Default for EventTracker<T> {
  fn default() -> Self {
    Self { events: Vec::new() }
  }
}

#[allow(dead_code)]
impl<T: Task> EventTracker<T> {
  #[inline]
  pub fn new() -> Self { Self::default() }

  #[inline]
  pub fn first(&self) -> Option<&Event<T>> { self.events.first() }
  #[inline]
  pub fn last(&self) -> Option<&Event<T>> { self.events.last() }
  #[inline]
  pub fn get(&self, index: usize) -> Option<&Event<T>> { self.events.get(index) }
  #[inline]
  pub fn get_from_end(&self, offset: usize) -> Option<&Event<T>> { self.events.get(self.events.len() - 1 - offset) }

  #[inline]
  pub fn contains(&self, f: impl FnMut(&Event<T>) -> bool) -> bool { self.events.iter().any(f) }
  #[inline]
  pub fn contains_no(&self, f: impl FnMut(&Event<T>) -> bool) -> bool { !self.contains(f) }
  #[inline]
  pub fn contains_count(&self, count: usize, f: impl FnMut(&&Event<T>) -> bool) -> bool { self.events.iter().filter(f).count() == count }
  #[inline]
  pub fn contains_one(&self, f: impl FnMut(&&Event<T>) -> bool) -> bool { self.contains_count(1, f) }

  #[inline]
  pub fn contains_one_execute_start(&self) -> bool {
    self.contains_one(|e| Self::match_execute_start(e))
  }
  #[inline]
  pub fn contains_no_execute_start(&self) -> bool {
    self.contains_no(|e| Self::match_execute_start(e))
  }
  #[inline]
  pub fn contains_one_execute_start_of(&self, task: &T) -> bool {
    self.contains_one(|e| Self::match_execute_start_of(e, task))
  }
  #[inline]
  pub fn contains_no_execute_start_of(&self, task: &T) -> bool {
    self.contains_no(|e| Self::match_execute_start_of(e, task))
  }

  #[inline]
  pub fn contains_one_execute_end(&self) -> bool {
    self.contains_one(|e| Self::match_execute_end(e))
  }
  #[inline]
  pub fn contains_no_execute_end(&self) -> bool {
    self.contains_no(|e| Self::match_execute_end(e))
  }
  #[inline]
  pub fn contains_one_execute_end_of(&self, task: &T) -> bool {
    self.contains_one(|e| Self::match_execute_end_of(e, task))
  }
  #[inline]
  pub fn contains_no_execute_end_of(&self, task: &T) -> bool {
    self.contains_no(|e| Self::match_execute_end_of(e, task))
  }
  #[inline]
  pub fn contains_one_execute_end_of_with(&self, task: &T, output: &T::Output) -> bool {
    self.contains_one(|e| Self::match_execute_end_of_with(e, task, output))
  }
  #[inline]
  pub fn contains_no_execute_end_of_with(&self, task: &T, output: &T::Output) -> bool {
    self.contains_no(|e| Self::match_execute_end_of_with(e, task, output))
  }

  #[inline]
  pub fn get_index_of(&self, f: impl FnMut(&Event<T>) -> bool) -> Option<usize> {
    self.events.iter().position(f)
  }
  #[inline]
  pub fn get_index_of_execute_start_of(&self, task: &T) -> Option<usize> {
    self.get_index_of(|e| Self::match_execute_start_of(e, task))
  }
  #[inline]
  pub fn get_index_of_execute_end_of(&self, task: &T) -> Option<usize> {
    self.get_index_of(|e| Self::match_execute_end_of(e, task))
  }
  #[inline]
  pub fn get_index_of_execute_end_of_with(&self, task: &T, output: &T::Output) -> Option<usize> {
    self.get_index_of(|e| Self::match_execute_end_of_with(e, task, output))
  }

  #[inline]
  fn match_execute_start(e: &Event<T>) -> bool {
    match e {
      Event::ExecuteTaskStart(_) => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_start_of(e: &Event<T>, task: &T) -> bool {
    match e {
      Event::ExecuteTaskStart(t) if t == task => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_end(e: &Event<T>) -> bool {
    match e {
      Event::ExecuteTaskEnd(_, _) => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_end_of(e: &Event<T>, task: &T) -> bool {
    match e {
      Event::ExecuteTaskEnd(t, _) if t == task => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_end_of_with(e: &Event<T>, task: &T, output: &T::Output) -> bool {
    match e {
      Event::ExecuteTaskEnd(t, o) if t == task && o == output => true,
      _ => false,
    }
  }

  #[inline]
  pub fn iter_events(&self) -> impl Iterator<Item=&Event<T>> { self.events.iter() }
  #[inline]
  pub fn take_events(&mut self) -> Vec<Event<T>> { std::mem::take(&mut self.events) }
  #[inline]
  pub fn clear(&mut self) { self.events.clear(); }
}

impl<T: Task> Tracker<T> for EventTracker<T> {
  #[inline]
  fn require_file(&mut self, file: &PathBuf) {
    self.events.push(Event::RequireFile(file.clone()));
  }
  #[inline]
  fn provide_file(&mut self, file: &PathBuf) {
    self.events.push(Event::ProvideFile(file.clone()));
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
  fn require_top_down_initial_start(&mut self, _task: &T) {}
  #[inline]
  fn check_top_down_start(&mut self, _task: &T) {}
  #[inline]
  fn check_require_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_require_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &dyn Error>) {}
  #[inline]
  fn check_provide_file_start(&mut self, _dependency: &FileDependency) {}
  #[inline]
  fn check_provide_file_end(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &dyn Error>) {}
  #[inline]
  fn check_require_task_start(&mut self, _dependency: &TaskDependency<T, T::Output>) {}
  #[inline]
  fn check_require_task_end(&mut self, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<&OutputStamp<T::Output>>) {}
  #[inline]
  fn check_top_down_end(&mut self, _task: &T) {}
  #[inline]
  fn require_top_down_initial_end(&mut self, _task: &T, _output: &T::Output) {}

  #[inline]
  fn update_affected_by_start<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, _changed_files: I) {}
  #[inline]
  fn update_affected_by_end(&mut self) {}
  #[inline]
  fn schedule_affected_by_file_start(&mut self, _file: &PathBuf) {}
  #[inline]
  fn check_affected_by_file(&mut self, _dependency: &FileDependency, _inconsistent: Result<Option<&FileStamp>, &dyn Error>) {}
  #[inline]
  fn schedule_affected_by_file_end(&mut self, _file: &PathBuf) {}
  #[inline]
  fn check_affected_by_task_start(&mut self, _task: &T) {}
  #[inline]
  fn check_affected_by_require_task(&mut self, _dependency: &TaskDependency<T, T::Output>, _inconsistent: Option<&OutputStamp<T::Output>>) {}
  #[inline]
  fn check_affected_by_task_end(&mut self, _task: &T) {}
  #[inline]
  fn schedule_task(&mut self, _task: &T) {}
}
