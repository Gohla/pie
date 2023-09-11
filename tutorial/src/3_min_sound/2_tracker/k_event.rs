use std::path::{Path, PathBuf};

use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};
use crate::Task;
use crate::tracker::Tracker;

/// [`Tracker`] that stores [events](Event) in a [`Vec`], useful in testing to assert that a context implementation is 
/// incremental and sound.
#[derive(Clone, Debug)]
pub struct EventTracker<T: Task> {
  events: Vec<Event<T>>,
}

impl<T: Task> Default for EventTracker<T> {
  fn default() -> Self { Self { events: Vec::new() } }
}

/// Enumeration of important build events.
#[derive(Debug, Clone)]
pub enum Event<T: Task> {
  /// A file at `path` has been required, using `stamper` to create `stamp`.
  RequiredFile { path: PathBuf, stamper: FileStamper, stamp: FileStamp },
  /// Require `task` using `stamper`.
  RequireTask { task: T, stamper: OutputStamper },
  /// A `task` has been required, resulting in consistent `output`, using `stamper` to create `stamp`, and task 
  /// `was_executed`.
  RequiredTask { task: T, output: T::Output, stamper: OutputStamper, stamp: OutputStamp<T::Output>, was_executed: bool },

  /// Execute `task`.
  Execute { task: T },
  /// A `task` has been executed, producing `output`.
  Executed { task: T, output: T::Output },
}
