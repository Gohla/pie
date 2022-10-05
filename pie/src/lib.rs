use std::any::Any;
use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{BuildHasher, Hash};
use std::path::PathBuf;

use serde::de::DeserializeOwned;
use serde::Serialize;

use pie_tagged_serde::Id;

use crate::runner::TopDownRunner;
use crate::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};
use crate::trait_object::{DynOutput, DynTask};

pub mod prelude;
pub mod dependency;
pub mod runner;
pub mod store;
pub mod tracker;
pub mod task;
pub mod trait_object;

/// The unit of computation in a programmatic incremental build system.
pub trait Task: Eq + Hash + Clone + Id + Serialize + DeserializeOwned + Any + Debug {
  /// The type of output this task produces when executed. Must implement [`Eq`], [`Clone`], and either not contain any 
  /// references, or only `'static` references.
  type Output: Output;
  /// Execute the task, with `context` providing a means to specify dependencies, producing an instance of 
  /// `Self::Output`.
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;

  /// Convert this task into a trait-object-safe version of this trait.
  #[inline]
  fn as_dyn(&self) -> &dyn DynTask { self as &dyn DynTask }
  /// Downcasts the given reference to a trait-object-safe output into an output of this task's output type.
  #[inline]
  fn downcast_ref_output(output: &Box<dyn DynOutput>) -> Option<&Self::Output> {
    // Note: `output.as_ref` is very important here, because `Box<dyn DynOutput>` also implements `DynOutput`, which 
    // in turn has an `as_any` method as well. However, `downcast_ref` will *always fail* on `Box<dyn DynOutput>` 
    // because it will try to downcast the box instead of what is inside the box.
    output.as_ref().as_any().downcast_ref::<Self::Output>()
  }
  /// Downcasts the given mutable reference to a trait-object-safe output into an output of this task's output type.
  #[inline]
  fn downcast_mut_output(output: &mut Box<dyn DynOutput>) -> Option<&mut Self::Output> {
    // Note: `output.as_mut` is very important here, for the same reason as in `downcast_ref_output`.
    output.as_mut().as_any_mut().downcast_mut::<Self::Output>()
  }
}


/// Trait alias for task outputs.
pub trait Output: Eq + Clone + Id + Serialize + DeserializeOwned + Any + Debug {}

impl<T: Eq + Clone + Id + Serialize + DeserializeOwned + Any + Debug> Output for T {}


/// Incremental context, mediating between tasks and executors, enabling tasks to dynamically create dependencies that 
/// executors check for consistency and use in incremental execution.
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


/// Main entry point into the PIE build system.
#[derive(Debug)]
pub struct Pie<A = NoopTracker, H = RandomState> {
  store: Store<H>,
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

  /// Creates a new [`Pie`] instance with given `store`.
  #[inline]
  pub fn with_store(store: Store<RandomState>) -> Self { Self { store, tracker: NoopTracker::default() } }
}

impl<A: Tracker + Default> Pie<A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn with_tracker(tracker: A) -> Self { Self { store: Store::default(), tracker } }

  /// Creates a new [`Pie`] instance with given `store` and `tracker`.
  #[inline]
  pub fn with(store: Store<RandomState>, tracker: A) -> Self { Self { store, tracker } }
}

impl<A: Tracker + Default, H: BuildHasher + Default> Pie<A, H> {
  /// Creates a new build session. Only one session may be active at once, enforced via mutable (exclusive) borrow.
  #[inline]
  pub fn new_session(&mut self) -> Session<A, H> { Session::new(self) }
  /// Runs `f` inside a new session.
  #[inline]
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session<A, H>) -> R) -> R {
    let session = self.new_session();
    f(session)
  }

  /// Gets the [`Store`] instance.
  #[inline]
  pub fn store(&self) -> &Store<H> { &self.store }

  /// Gets the [`Tracker`] instance.
  #[inline]
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }

  /// Creates a new [`Pie`] instance with the store replaced by the given `store`.
  #[inline]
  pub fn replace_store(self, store: Store<H>) -> Self {
    Self { store, tracker: self.tracker }
  }
}


/// A session in which builds are executed. Every task is only executed once each session.
#[derive(Debug)]
pub struct Session<'p, A, H> {
  store: &'p mut Store<H>,
  tracker: &'p mut A,
  visited: HashSet<TaskNode, H>,
  dependency_check_errors: Vec<Box<dyn Error>>,
}

impl<'p, A: Tracker + Default, H: BuildHasher + Default> Session<'p, A, H> {
  #[inline]
  fn new(pie: &'p mut Pie<A, H>) -> Self {
    Self {
      store: &mut pie.store,
      tracker: &mut pie.tracker,
      visited: HashSet::default(),
      dependency_check_errors: Vec::default(),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require<T: Task>(&mut self, task: &T) -> T::Output {
    let mut runner = TopDownRunner::new(self);
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
