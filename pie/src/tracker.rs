use std::io;
use std::io::{Stderr, Stdout};
use std::path::PathBuf;

use crate::task::{DynOutput, DynOutputExt, DynTask, DynTaskExt};

/// Trait for tracking build events. Can be used to implement logging, event tracing, and possibly progress tracking.
pub trait Tracker {
  fn require_file(&mut self, file: &PathBuf);
  fn provide_file(&mut self, file: &PathBuf);
  fn require_task(&mut self, task: &dyn DynTask);

  fn execute_task_start(&mut self, task: &dyn DynTask);
  fn execute_task_end(&mut self, task: &dyn DynTask, output: &dyn DynOutput);
}


/// A [`Tracker`] that does nothing.
#[derive(Default, Clone, Debug)]
pub struct NoopTracker;

impl Tracker for NoopTracker {
  #[inline]
  fn require_file(&mut self, _file: &PathBuf) {}
  #[inline]
  fn provide_file(&mut self, _file: &PathBuf) {}
  #[inline]
  fn require_task(&mut self, _task: &dyn DynTask) {}

  #[inline]
  fn execute_task_start(&mut self, _task: &dyn DynTask) {}
  #[inline]
  fn execute_task_end(&mut self, _task: &dyn DynTask, _output: &dyn DynOutput) {}
}


/// A [`Tracker`] that forwards events to two other [`Tracker`]s.
#[derive(Default, Clone, Debug)]
pub struct CompositeTracker<T1, T2>(pub T1, pub T2);

impl<T1: Tracker, T2: Tracker> Tracker for CompositeTracker<T1, T2> {
  #[inline]
  fn require_file(&mut self, file: &PathBuf) {
    self.0.require_file(file);
    self.1.require_file(file);
  }
  #[inline]
  fn provide_file(&mut self, file: &PathBuf) {
    self.0.provide_file(file);
    self.1.provide_file(file);
  }
  #[inline]
  fn require_task(&mut self, task: &dyn DynTask) {
    self.0.require_task(task);
    self.1.require_task(task);
  }

  #[inline]
  fn execute_task_start(&mut self, task: &dyn DynTask) {
    self.0.execute_task_start(task);
    self.1.execute_task_start(task);
  }
  #[inline]
  fn execute_task_end(&mut self, task: &dyn DynTask, output: &dyn DynOutput) {
    self.0.execute_task_end(task, output);
    self.1.execute_task_end(task, output);
  }
}


/// A [`Tracker`] that writes events to a [`std::io::Write`] instance, for example [`std::io::Stdout`].
#[derive(Debug, Clone)]
pub struct WritingTracker<W> {
  writer: W,
}

impl Default for WritingTracker<Stdout> {
  #[inline]
  fn default() -> Self { Self::new_stdout_writer() }
}

impl Default for WritingTracker<Stderr> {
  #[inline]
  fn default() -> Self { Self::new_stderr_writer() }
}

impl<W: io::Write> WritingTracker<W> {
  #[inline]
  pub fn new(writer: W) -> Self { Self { writer } }
}

impl WritingTracker<Stdout> {
  #[inline]
  pub fn new_stdout_writer() -> Self { Self::new(io::stdout()) }
}

impl WritingTracker<Stderr> {
  #[inline]
  pub fn new_stderr_writer() -> Self { Self::new(io::stderr()) }
}

impl<W: io::Write> Tracker for WritingTracker<W> {
  #[inline]
  fn require_file(&mut self, file: &PathBuf) {
    writeln!(&mut self.writer, "Required file: {}", file.display()).ok();
  }
  #[inline]
  fn provide_file(&mut self, file: &PathBuf) {
    writeln!(&mut self.writer, "Provided file: {}", file.display()).ok();
  }
  #[inline]
  fn require_task(&mut self, task: &dyn DynTask) {
    writeln!(&mut self.writer, "Required task: {:?}", task).ok();
  }

  #[inline]
  fn execute_task_start(&mut self, task: &dyn DynTask) {
    writeln!(&mut self.writer, "Execute task start: {:?}", task).ok();
  }
  #[inline]
  fn execute_task_end(&mut self, task: &dyn DynTask, output: &dyn DynOutput) {
    writeln!(&mut self.writer, "Execute task end: {:?} => {:?}", task, output).ok();
  }
}


/// A [`Tracker`] that stores [`Event`]s in a [`Vec`], useful in testing situations where we check build events after
/// building. 
#[derive(Default, Clone, Debug)]
pub struct EventTracker {
  events: Vec<Event>,
}

#[derive(Debug, Clone)]
pub enum Event {
  RequireFile(PathBuf),
  ProvideFile(PathBuf),
  RequireTask(Box<dyn DynTask>),

  ExecuteTaskStart(Box<dyn DynTask>),
  ExecuteTaskEnd(Box<dyn DynTask>, Box<dyn DynOutput>),
}

#[allow(dead_code)]
impl EventTracker {
  #[inline]
  pub fn new() -> Self { Self::default() }

  #[inline]
  pub fn first(&self) -> Option<&Event> { self.events.first() }
  #[inline]
  pub fn last(&self) -> Option<&Event> { self.events.last() }
  #[inline]
  pub fn get(&self, index: usize) -> Option<&Event> { self.events.get(index) }
  #[inline]
  pub fn get_from_end(&self, offset: usize) -> Option<&Event> { self.events.get(self.events.len() - 1 - offset) }

  #[inline]
  pub fn contains(&self, f: impl FnMut(&Event) -> bool) -> bool { self.events.iter().any(f) }
  #[inline]
  pub fn contains_no(&self, f: impl FnMut(&Event) -> bool) -> bool { !self.contains(f) }
  #[inline]
  pub fn contains_count(&self, count: usize, f: impl FnMut(&&Event) -> bool) -> bool { self.events.iter().filter(f).count() == count }
  #[inline]
  pub fn contains_one(&self, f: impl FnMut(&&Event) -> bool) -> bool { self.contains_count(1, f) }

  #[inline]
  pub fn contains_one_execute_start(&self) -> bool {
    self.contains_one(|e| Self::match_execute_start(e))
  }
  #[inline]
  pub fn contains_no_execute_start(&self) -> bool {
    self.contains_no(|e| Self::match_execute_start(e))
  }
  #[inline]
  pub fn contains_one_execute_start_of(&self, task: &Box<dyn DynTask>) -> bool {
    self.contains_one(|e| Self::match_execute_start_of(e, task))
  }
  #[inline]
  pub fn contains_no_execute_start_of(&self, task: &Box<dyn DynTask>) -> bool {
    self.contains_no(|e| Self::match_execute_start_of(e, task))
  }

  #[inline]
  pub fn get_index_of(&self, f: impl FnMut(&Event) -> bool) -> Option<usize> {
    self.events.iter().position(f)
  }
  #[inline]
  pub fn get_index_of_execute_start_of(&self, task: &Box<dyn DynTask>) -> Option<usize> {
    self.get_index_of(|e| Self::match_execute_start_of(e, task))
  }
  #[inline]
  pub fn get_index_of_execute_end_of(&self, task: &Box<dyn DynTask>) -> Option<usize> {
    self.get_index_of(|e| Self::match_execute_end_of(e, task))
  }

  #[inline]
  fn match_execute_start(e: &Event) -> bool {
    match e {
      Event::ExecuteTaskStart(_) => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_start_of(e: &Event, task: &Box<dyn DynTask>) -> bool {
    match e {
      Event::ExecuteTaskStart(t) if t == task => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_end(e: &Event) -> bool {
    match e {
      Event::ExecuteTaskEnd(_, _) => true,
      _ => false,
    }
  }
  #[inline]
  fn match_execute_end_of(e: &Event, task: &Box<dyn DynTask>) -> bool {
    match e {
      Event::ExecuteTaskEnd(t, _) if t == task => true,
      _ => false,
    }
  }

  #[inline]
  pub fn iter_events(&self) -> impl Iterator<Item=&Event> { self.events.iter() }
  #[inline]
  pub fn take_events(&mut self) -> Vec<Event> { std::mem::take(&mut self.events) }
  #[inline]
  pub fn clear(&mut self) { self.events.clear(); }
}

impl Tracker for EventTracker {
  #[inline]
  fn require_file(&mut self, file: &PathBuf) {
    self.events.push(Event::RequireFile(file.clone()));
  }
  #[inline]
  fn provide_file(&mut self, file: &PathBuf) {
    self.events.push(Event::ProvideFile(file.clone()));
  }
  #[inline]
  fn require_task(&mut self, task: &dyn DynTask) {
    self.events.push(Event::RequireTask(task.clone()));
  }

  #[inline]
  fn execute_task_start(&mut self, task: &dyn DynTask) {
    self.events.push(Event::ExecuteTaskStart(task.clone()));
  }
  #[inline]
  fn execute_task_end(&mut self, task: &dyn DynTask, output: &dyn DynOutput) {
    self.events.push(Event::ExecuteTaskEnd(task.clone(), output.clone()));
  }
}
