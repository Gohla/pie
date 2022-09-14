use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::hash::BuildHasher;
use std::panic;
use std::path::PathBuf;

use task::Task;

use crate::runner::Runner;
use crate::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};

pub mod prelude;
pub mod task;
pub mod dependency;
pub mod runner;
pub mod store;
pub mod tracker;

/// Main entry point into the PIE build system.
#[derive(Debug)]
pub struct Pie<A = NoopTracker, H = RandomState, C = Runner<A, H>> {
  store: Store<C, H>,
  tracker: A,
}

impl Default for Pie {
  #[inline]
  fn default() -> Self { Self { store: Store::default(), tracker: NoopTracker::default() } }
}

impl Pie {
  /// Creates a new [`Pie`] instance.
  #[inline]
  pub fn new() -> Self { Self::default() }
}

impl<A: Tracker + Default> Pie<A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn with_tracker(tracker: A) -> Self { Self { store: Store::default(), tracker } }
}

impl<A: Tracker + Default, H: BuildHasher + Default> Pie<A, H> {
  /// Creates a new build session. Only one session may be active at once. The returned session must be dropped before 
  /// creating a new session. 
  #[inline]
  pub fn new_session(&mut self) -> Session<A, H> { Session::new(self) }
  /// Runs `f` inside a new session.
  #[inline]
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session<A, H>) -> R) -> R {
    let session = self.new_session();
    f(session)
  }

  /// Gets the [`Tracker`] instance.
  #[inline]
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }
}


/// A session in which builds occur. Every task is only executed once each session.
#[derive(Debug)]
pub struct Session<'p, A, H, C = Runner<A, H>> {
  pie: &'p mut Pie<A, H, C>,
  data: Option<SessionData<A, H, C>>,
}

/// Internal session data.
#[derive(Debug)]
pub struct SessionData<A, H, C> {
  store: Store<C, H>,
  tracker: A,
  visited: HashSet<TaskNode, H>,
  dependency_check_errors: Vec<Box<dyn Error>>,
}

impl<'p, A: Tracker + Default, H: BuildHasher + Default> Session<'p, A, H> {
  #[inline]
  fn new(pie: &'p mut Pie<A, H>) -> Self {
    let store = std::mem::take(&mut pie.store);
    let tracker = std::mem::take(&mut pie.tracker);
    let data = Some(SessionData {
      store,
      tracker,
      visited: HashSet::default(),
      dependency_check_errors: Vec::default(),
    });
    Self { pie, data }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require<T: Task>(&mut self, task: &T) -> T::Output {
    let data = self.data.take().unwrap();
    let mut runner = Runner::new(data);
    let result = panic::catch_unwind(panic::AssertUnwindSafe(|| { // Catch panic (if it unwinds)
      runner.require(task)
    }));
    self.data = Some(runner.into_data()); // Restore data even after panic
    match result {
      Ok(output) => output,
      Err(e) => panic::resume_unwind(e) // Resume unwinding panic
    }
  }

  /// Gets the [`Tracker`] instance.
  #[inline]
  pub fn tracker(&self) -> &A { &self.data.as_ref().unwrap().tracker }
  /// Gets the mutable [`Tracker`] instance.
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.data.as_mut().unwrap().tracker }

  /// Gets a slice over all errors produced during dependency checks.
  #[inline]
  pub fn dependency_check_errors(&self) -> &[Box<dyn Error>] {
    &self.data.as_ref().unwrap().dependency_check_errors
  }
}

impl<'p, A, H, C> Drop for Session<'p, A, H, C> {
  fn drop(&mut self) {
    let data = self.data.take().unwrap();
    self.pie.store = data.store;
    self.pie.tracker = data.tracker;
  }
}


/// Incremental context, mediating between tasks and runners, enabling tasks to dynamically create dependencies that 
/// runners check for consistency and use in incremental execution.
pub trait Context {
  /// Requires given `task`, creating a dependency to it, and returning its up-to-date output.
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output;
  /// Requires file at given `path`, creating a read-dependency to the file by reading its content or metadata at the 
  /// time this function is called, and returning the opened file. Call this method *before reading from the file*.
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error>;
  /// Provides file at given `path`, creating a write-dependency to it by writing to its content or changing its
  /// metadata at the time this function is called. Call this method *after writing to the file*. This method does not 
  /// return the opened file, as it must be called *after writing to the file*.
  fn provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error>;
}
