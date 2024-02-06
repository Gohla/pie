use std::fmt::Debug;
use std::ops::RangeInclusive;

use crate::Task;
use crate::tracker::Tracker;
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::task::OutputCheckerObj;

/// A [`Tracker`] that stores [`Event`]s in a [`Vec`], useful in testing to assert that a context implementation is
/// incremental and correct.
#[derive(Clone, Debug)]
pub struct EventTracker {
  events: Vec<Event>,
  clear_on_build_start: bool,
}

impl Default for EventTracker {
  fn default() -> Self {
    Self { events: Vec::new(), clear_on_build_start: true }
  }
}

/// Enumeration of important build events.
#[derive(Clone, Debug)]
pub enum Event {
  BuildStart,
  BuildEnd,

  RequireStart(RequireStart),
  RequireEnd(RequireEnd),

  ReadStart(ResourceStart),
  ReadEnd(ResourceEnd),
  WriteStart(ResourceStart),
  WriteEnd(ResourceEnd),

  ExecuteStart(ExecuteStart),
  ExecuteEnd(ExecuteEnd),
}

/// Trait for access to tasks in specific kinds of [`Event`]s.
pub trait TaskAccess {
  fn task(&self) -> &dyn KeyObj;
  #[inline]
  fn task_downcast<T: Task>(&self) -> Option<&T> {
    self.task().as_any().downcast_ref()
  }
  #[inline]
  fn task_equals<T: Task>(&self, task: &T) -> bool {
    self.task_downcast::<T>().map(|t| t == task).unwrap_or_default()
  }
}

/// Start: require `task` using `checker`.
#[derive(Clone, Debug)]
pub struct RequireStart {
  pub task: Box<dyn KeyObj>,
  pub checker: Box<dyn OutputCheckerObj>,
  pub index: usize,
}
impl TaskAccess for RequireStart {
  #[inline]
  fn task(&self) -> &dyn KeyObj { self.task.as_ref() }
}
/// End: required `task`, using `checker` to create `stamp`, resulting in `output`.
#[derive(Clone, Debug)]
pub struct RequireEnd {
  pub task: Box<dyn KeyObj>,
  pub checker: Box<dyn OutputCheckerObj>,
  pub stamp: Box<dyn ValueObj>,
  pub output: Box<dyn ValueObj>,
  pub index: usize,
}
impl TaskAccess for RequireEnd {
  #[inline]
  fn task(&self) -> &dyn KeyObj { self.task.as_ref() }
}
/// Start: read/write `resource` using `checker`.
#[derive(Clone, Debug)]
pub struct ResourceStart {
  pub resource: Box<dyn KeyObj>,
  pub checker: Box<dyn ValueObj>,
  pub index: usize,
}
/// End: read/written `resource` using `checker` to create `stamp`.
#[derive(Clone, Debug)]
pub struct ResourceEnd {
  pub resource: Box<dyn KeyObj>,
  pub checker: Box<dyn ValueObj>,
  pub stamp: Box<dyn ValueObj>,
  pub index: usize,
}
/// Start: execute `task`.
#[derive(Clone, Debug)]
pub struct ExecuteStart {
  pub task: Box<dyn KeyObj>,
  pub index: usize,
}
impl TaskAccess for ExecuteStart {
  #[inline]
  fn task(&self) -> &dyn KeyObj { self.task.as_ref() }
}
/// End: executed `task`, producing `output`.
#[derive(Clone, Debug)]
pub struct ExecuteEnd {
  pub task: Box<dyn KeyObj>,
  pub output: Box<dyn ValueObj>,
  pub index: usize,
}
impl TaskAccess for ExecuteEnd {
  #[inline]
  fn task(&self) -> &dyn KeyObj { self.task.as_ref() }
}

impl Tracker for EventTracker {
  #[inline]
  fn build_start(&mut self) {
    if self.clear_on_build_start {
      self.events.clear();
    }
    self.events.push(Event::BuildStart);
  }
  #[inline]
  fn build_end(&mut self) {
    self.events.push(Event::BuildEnd);
  }

  #[inline]
  fn require_start(&mut self, task: &dyn KeyObj, checker: &dyn OutputCheckerObj) {
    let data = RequireStart {
      task: task.to_owned(),
      checker: checker.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::RequireStart(data));
  }
  #[inline]
  fn require_end(
    &mut self,
    task: &dyn KeyObj,
    checker: &dyn OutputCheckerObj,
    stamp: &dyn ValueObj,
    output: &dyn ValueObj,
  ) {
    let data = RequireEnd {
      task: task.to_owned(),
      checker: checker.to_owned(),
      stamp: stamp.to_owned(),
      output: output.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::RequireEnd(data));
  }

  #[inline]
  fn read_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj) {
    let data = ResourceStart {
      resource: resource.to_owned(),
      checker: checker.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::ReadStart(data));
  }
  #[inline]
  fn read_end(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {
    let data = ResourceEnd {
      resource: resource.to_owned(),
      checker: checker.to_owned(),
      stamp: stamp.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::ReadEnd(data));
  }

  #[inline]
  fn write_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj) {
    let data = ResourceStart {
      resource: resource.to_owned(),
      checker: checker.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::WriteStart(data));
  }
  #[inline]
  fn write_end(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {
    let data = ResourceEnd {
      resource: resource.to_owned(),
      checker: checker.to_owned(),
      stamp: stamp.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::WriteEnd(data));
  }

  #[inline]
  fn execute_start(&mut self, task: &dyn KeyObj) {
    let data = ExecuteStart {
      task: task.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::ExecuteStart(data));
  }
  #[inline]
  fn execute_end(&mut self, task: &dyn KeyObj, output: &dyn ValueObj) {
    let data = ExecuteEnd {
      task: task.to_owned(),
      output: output.to_owned(),
      index: self.events.len(),
    };
    self.events.push(Event::ExecuteEnd(data));
  }
}

impl Event {
  /// Returns `true` if this is a [build start event](Event::BuildStart).
  pub fn is_build_start(&self) -> bool {
    match self {
      Event::BuildStart => true,
      _ => false,
    }
  }
  /// Returns `true` if this is a [build end event](Event::BuildEnd).
  pub fn is_build_end(&self) -> bool {
    match self {
      Event::BuildStart => true,
      _ => false,
    }
  }

  /// Returns `Some(&data)` if this is a [require start event](Event::RequireTaskStart) for `task`, or `None` otherwise.
  pub fn match_require_start(&self, task: &dyn KeyObj) -> Option<&RequireStart> {
    match self {
      Event::RequireStart(data) if data.task.as_ref() == task => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [require end event](Event::RequireTaskStart) for `task`, or `None` otherwise.
  pub fn match_require_end(&self, task: &dyn KeyObj) -> Option<&RequireEnd> {
    match self {
      Event::RequireEnd(data) if data.task.as_ref() == task => Some(data),
      _ => None,
    }
  }

  /// Returns `Some(&data)` if this is a [read start event](Event::ReadStart) for `resource`, or `None` otherwise.
  pub fn match_read_start(&self, resource: &dyn KeyObj) -> Option<&ResourceStart> {
    match self {
      Event::ReadStart(data) if data.resource.as_ref() == resource => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [read end event](Event::ReadEnd) for `resource`, or `None` otherwise.
  pub fn match_read_end(&self, resource: &dyn KeyObj) -> Option<&ResourceEnd> {
    match self {
      Event::ReadEnd(data) if data.resource.as_ref() == resource => Some(data),
      _ => None,
    }
  }

  /// Returns `Some(&data)` if this is a [write start event](Event::WriteStart) for `resource`, or `None` otherwise.
  pub fn match_write_start(&self, resource: &dyn KeyObj) -> Option<&ResourceStart> {
    match self {
      Event::WriteStart(data) if data.resource.as_ref() == resource => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [write end event](Event::WriteEnd) for `resource`, or `None` otherwise.
  pub fn match_write_end(&self, resource: &dyn KeyObj) -> Option<&ResourceEnd> {
    match self {
      Event::WriteEnd(data) if data.resource.as_ref() == resource => Some(data),
      _ => None,
    }
  }

  /// Returns `true` if this is an execute [start](Event::ExecuteStart) or [end](Event::ExecuteEnd) event.
  pub fn is_execute(&self) -> bool {
    match self {
      Event::ExecuteStart(_) | Event::ExecuteEnd(_) => true,
      _ => false,
    }
  }
  /// Returns `true` if this is an execute [start](Event::ExecuteStart) or [end](Event::ExecuteEnd) event for `task`.
  pub fn is_execute_of(&self, task: &dyn KeyObj) -> bool {
    match self {
      Event::ExecuteStart(ExecuteStart { task: t, .. }) |
      Event::ExecuteEnd(ExecuteEnd { task: t, .. }) if t.as_ref() == task => true,
      _ => false,
    }
  }
  /// Returns `Some(&data)` if this is an [execute start event](Event::ExecuteStart) for `task`, or `None` otherwise.
  pub fn match_execute_start(&self, task: &dyn KeyObj) -> Option<&ExecuteStart> {
    match self {
      Event::ExecuteStart(data) if data.task.as_ref() == task => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is an [execute end event](Event::ExecuteStart) for `task`, or `None` otherwise.
  pub fn match_execute_end(&self, task: &dyn KeyObj) -> Option<&ExecuteEnd> {
    match self {
      Event::ExecuteEnd(data) if data.task.as_ref() == task => Some(data),
      _ => None,
    }
  }
}

impl EventTracker {
  /// Returns a slice over all events.
  pub fn slice(&self) -> &[Event] {
    &self.events
  }
  /// Returns an iterator over all events.
  pub fn iter(&self) -> impl Iterator<Item=&Event> {
    self.events.iter()
  }

  /// Returns `true` if `predicate` returns `true` for any event.
  pub fn any(&self, predicate: impl FnMut(&Event) -> bool) -> bool {
    self.iter().any(predicate)
  }
  /// Returns `true` if `predicate` returns `true` for exactly one event.
  pub fn one(&self, predicate: impl FnMut(&&Event) -> bool) -> bool {
    self.iter().filter(predicate).count() == 1
  }

  /// Returns `Some(v)` for the first event `e` where `f(e)` returns `Some(v)`, or `None` otherwise.
  pub fn find_map<R>(&self, f: impl FnMut(&Event) -> Option<&R>) -> Option<&R> {
    self.iter().find_map(f)
  }


  /// Finds the first require [start](Event::RequireStart) and [end](Event::RequireEnd) event for `task` and returns
  /// `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_require(&self, task: &dyn KeyObj) -> Option<(&RequireStart, &RequireEnd)> {
    let start_data = self.find_map(|e| e.match_require_start(task));
    let end_data = self.find_map(|e| e.match_require_end(task));
    start_data.zip(end_data)
  }
  /// Finds the first require [start](Event::RequireStart) and [end](Event::RequireEnd) event for `task` and
  /// returns `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_require_range(&self, task: &dyn KeyObj) -> Option<RangeInclusive<usize>> {
    self.first_require(task).map(|(s, e)| s.index..=e.index)
  }

  /// Finds the first read [start](Event::ReadStart) and [end](Event::ReadEnd) event for `resource` and returns
  /// `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_read(&self, resource: &dyn KeyObj) -> Option<(&ResourceStart, &ResourceEnd)> {
    let start_data = self.find_map(|e| e.match_read_start(resource));
    let end_data = self.find_map(|e| e.match_read_end(resource));
    start_data.zip(end_data)
  }
  /// Finds the first read [start](Event::ReadStart) and [end](Event::ReadEnd) event for `resource` and returns
  /// `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_read_range(&self, resource: &dyn KeyObj) -> Option<RangeInclusive<usize>> {
    self.first_read(resource).map(|(s, e)| s.index..=e.index)
  }
  /// Finds the first [read end event](Event::ReadEnd) for `resource` and returns `Some(&data)`, or `None` otherwise.
  pub fn first_read_end(&self, resource: &dyn KeyObj) -> Option<&ResourceEnd> {
    self.find_map(|e| e.match_read_end(resource))
  }
  /// Finds the first [read end event](Event::ReadEnd) for `resource` and returns `Some(&data.index)`, or `None`
  /// otherwise.
  pub fn first_read_end_index(&self, resource: &dyn KeyObj) -> Option<&usize> {
    self.first_read_end(resource).map(|d| &d.index)
  }

  /// Finds the first write [start](Event::WriteStart) and [end](Event::WriteEnd) event for `resource` and returns
  /// `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_write(&self, resource: &dyn KeyObj) -> Option<(&ResourceStart, &ResourceEnd)> {
    let start_data = self.find_map(|e| e.match_write_start(resource));
    let end_data = self.find_map(|e| e.match_write_end(resource));
    start_data.zip(end_data)
  }
  /// Finds the first write [start](Event::WriteStart) and [end](Event::WriteEnd) event for `resource` and returns
  /// `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_write_range(&self, resource: &dyn KeyObj) -> Option<RangeInclusive<usize>> {
    self.first_write(resource).map(|(s, e)| s.index..=e.index)
  }
  /// Finds the first [write end event](Event::WriteEnd) for `resource` and returns `Some(&data)`, or `None` otherwise.
  pub fn first_write_end(&self, resource: &dyn KeyObj) -> Option<&ResourceEnd> {
    self.find_map(|e| e.match_write_end(resource))
  }
  /// Finds the first [write end event](Event::WriteEnd) for `resource` and returns `Some(&data.index)`, or `None`
  /// otherwise.
  pub fn first_write_end_index(&self, resource: &dyn KeyObj) -> Option<&usize> {
    self.first_write_end(resource).map(|d| &d.index)
  }

  /// Returns `true` if any task was executed.
  pub fn any_execute(&self) -> bool {
    self.any(|e| e.is_execute())
  }
  /// Returns `true` if `task` was executed.
  pub fn any_execute_of(&self, task: &dyn KeyObj) -> bool {
    self.any(|e| e.is_execute_of(task))
  }
  /// Returns `true` if `task` was executed exactly once.
  pub fn one_execute_of(&self, task: &dyn KeyObj) -> bool {
    self.one(|e| e.match_execute_start(task).is_some())
  }
  /// Finds the first execute [start](Event::ExecuteStart) and [end](Event::ExecuteEnd) event for `task` and returns
  /// `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_execute(&self, task: &dyn KeyObj) -> Option<(&ExecuteStart, &ExecuteEnd)> {
    let start_data = self.find_map(|e| e.match_execute_start(task));
    let end_data = self.find_map(|e| e.match_execute_end(task));
    start_data.zip(end_data)
  }
  /// Finds the first execute [start](Event::ExecuteStart) and [end](Event::ExecuteEnd) event for `task` and returns
  /// `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_execute_range(&self, task: &dyn KeyObj) -> Option<RangeInclusive<usize>> {
    self.first_execute(task).map(|(s, e)| s.index..=e.index)
  }
  /// Finds the first [execute end event](Event::ExecuteEnd) for `task` and returns `Some(&data)`, or `None` otherwise.
  pub fn first_execute_end(&self, task: &dyn KeyObj) -> Option<&ExecuteEnd> {
    self.find_map(|e| e.match_execute_end(task))
  }
  /// Finds the first [execute end event](Event::ExecuteEnd) for `task` and returns `Some(&data.index)`, or `None`
  /// otherwise.
  pub fn first_execute_end_index(&self, task: &dyn KeyObj) -> Option<&usize> {
    self.first_execute_end(task).map(|d| &d.index)
  }
}
