use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{BuildHasher, Hash};
use std::path::PathBuf;

use crate::context::IncrementalTopDownContext;
use crate::dependency::FileStamper;
use crate::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};

pub mod dependency;
pub mod context;
pub mod store;
pub mod tracker;

/// The unit of computation in a programmatic incremental build system.
pub trait Task: Clone + Eq + Hash + Debug {
  /// The type of output this task produces when executed. Must implement [`Eq`], [`Clone`], and either not contain any 
  /// references, or only `'static` references.
  type Output: Output;
  /// Execute the task, with `context` providing a means to specify dependencies, producing an instance of 
  /// `Self::Output`.
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}


/// Trait alias for task outputs.
pub trait Output: Clone + Eq + Debug {}

impl<T: Clone + Eq + Debug> Output for T {}


/// Incremental context, mediating between tasks and executors, enabling tasks to dynamically create dependencies that 
/// executors check for consistency and use in incremental execution.
pub trait Context<T: Task> {
  /// Requires given `task`, creating a dependency to it, and returning its up-to-date output.
  fn require_task(&mut self, task: &T) -> T::Output;
  /// Requires file at given `path`, creating a read-dependency to the file by reading its content or metadata at the 
  /// time this function is called, and returning the opened file. Call this method *before reading from the file*.
  fn require_file(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<File, std::io::Error>;
  /// Provides file at given `path`, creating a write-dependency to it by writing to its content or changing its
  /// metadata at the time this function is called. Call this method *after writing to the file*. This method does not 
  /// return the opened file, as it must be called *after writing to the file*.
  fn provide_file(&mut self, path: &PathBuf, stamper: FileStamper) -> Result<(), std::io::Error>;
}


/// Main entry point into the PIE build system.
#[derive(Debug)]
pub struct Pie<T: Task, A = NoopTracker<T>, H = RandomState> {
  store: Store<T, H>,
  tracker: A,
}

impl<T: Task> Default for Pie<T> {
  #[inline]
  fn default() -> Self { Self { store: Store::default(), tracker: NoopTracker::default() } }
}

impl<T: Task> Pie<T> {
  /// Creates a new [`Pie`] instance.
  #[inline]
  pub fn new() -> Self { Self::default() }
}

impl<T: Task, A: Tracker<T> + Default> Pie<T, A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn with_tracker(tracker: A) -> Self { Self { store: Store::default(), tracker } }
}

impl<T: Task, A: Tracker<T> + Default, H: BuildHasher + Default> Pie<T, A, H> {
  /// Creates a new build session. Only one session may be active at once, enforced via mutable (exclusive) borrow.
  #[inline]
  pub fn new_session(&mut self) -> Session<T, A, H> { Session::new(self) }
  /// Runs `f` inside a new session.
  #[inline]
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session<T, A, H>) -> R) -> R {
    let session = self.new_session();
    f(session)
  }

  /// Gets the [`Tracker`] instance.
  #[inline]
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }

  /// Serializes the state with the given `serializer`.
  pub fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> where
    T: serde::Serialize,
    T::Output: serde::Serialize,
  {
    use serde::Serialize;
    self.store.serialize(serializer)
  }
  /// Deserializes the state from the given `deserializer`, and returns a new PIE instance with the deserialized state.
  pub fn deserialize<'de, D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self, D::Error> where
    T: serde::Deserialize<'de>,
    T::Output: serde::Deserialize<'de>,
  {
    use serde::Deserialize;
    let store = Store::deserialize(deserializer)?;
    Ok(Self { store, tracker: self.tracker })
  }
}


/// A session in which builds are executed. Every task is only executed once each session.
#[derive(Debug)]
pub struct Session<'p, T: Task, A, H> {
  store: &'p mut Store<T, H>,
  tracker: &'p mut A,
  visited: HashSet<TaskNode, H>,
  dependency_check_errors: Vec<Box<dyn Error>>,
}

impl<'p, T: Task, A: Tracker<T> + Default, H: BuildHasher + Default> Session<'p, T, A, H> {
  #[inline]
  fn new(pie: &'p mut Pie<T, A, H>) -> Self {
    Self {
      store: &mut pie.store,
      tracker: &mut pie.tracker,
      visited: HashSet::default(),
      dependency_check_errors: Vec::default(),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require(&mut self, task: &T) -> T::Output {
    let mut runner = IncrementalTopDownContext::new(self);
    runner.require(task)
  }

  /// Gets the [`Tracker`] instance.
  #[inline]
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }
  /// Gets a slice over all errors produced during dependency checks.
  #[inline]
  pub fn dependency_check_errors(&self) -> &[Box<dyn Error>] { &self.dependency_check_errors }
}
