use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{BuildHasher, Hash};
use std::io;
use std::path::{Path, PathBuf};

use stamp::{FileStamper, OutputStamper};

use crate::context::bottom_up::BottomUpContext;
use crate::context::top_down::TopDownContext;
use crate::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};

pub mod stamp;
pub mod tracker;
mod dependency;
mod store;
mod context;
mod fs;

/// The unit of computation in a programmatic incremental build system.
pub trait Task: Clone + Eq + Hash + Debug {
  /// The type of output this task produces when executed.
  type Output: Clone + Eq + Debug;
  /// Execute the task, with `context` providing a means to specify dynamic dependencies, returning `Self::Output`.
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}

/// Incremental context, mediating between tasks and executors, enabling tasks to dynamically create dependencies that 
/// executors use for incremental execution.
pub trait Context<T: Task> {
  /// Requires given `task`, creating a dependency to it with the default output stamper, returning its up-to-date 
  /// output.
  #[inline]
  fn require_task(&mut self, task: &T) -> T::Output {
    self.require_task_with_stamper(task, self.default_output_stamper())
  }
  /// Requires given `task`, creating a dependency to it with given `stamper`, returning its up-to-date output.
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output;

  /// Requires file at given `path`, creating a read-dependency to it by creating a stamp with the default require file 
  /// stamper. Returns the opened file (in read-only mode). Call this method *just before reading from the file*, so 
  /// that the stamp corresponds to the data that you are reading.
  #[inline]
  fn require_file(&mut self, path: impl AsRef<Path>) -> Result<Option<File>, io::Error> {
    self.require_file_with_stamper(path, self.default_require_file_stamper())
  }
  /// Requires file at given `path`, creating a read-dependency to it by creating a stamp with given `stamper`. 
  /// Returns the opened file (in read-only mode) if the file exists, `None` otherwise. Call this method *just before 
  /// reading from the file*, so that the stamp corresponds to the data that you are reading.
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error>;

  /// Provides file at given `path`, creating a write-dependency to it by creating a stamp with the default provide file
  /// stamper. Call this method *after writing to the file*, so that the stamp corresponds to the data that you've
  /// written to the file. This method does not return the opened file, as it must be called *after writing to the file*.
  #[inline]
  fn provide_file(&mut self, path: impl AsRef<Path>) -> Result<(), io::Error> {
    self.provide_file_with_stamper(path, self.default_provide_file_stamper())
  }
  /// Provides file at given `path`, creating a write-dependency to it by creating a stamp with given `stamper`.
  /// Call this method *after writing to the file*, so that the stamp corresponds to the data that you've written to the
  /// file. This method does not return the opened file, as it must be called *after writing to the file*.
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), io::Error>;

  /// Returns the default output stamper.
  fn default_output_stamper(&self) -> OutputStamper;
  /// Returns the default require file stamper.
  fn default_require_file_stamper(&self) -> FileStamper;
  /// Returns the default provide file stamper.
  fn default_provide_file_stamper(&self) -> FileStamper;
}


/// Main entry point into the PIE build system.
pub struct Pie<T: Task, A = NoopTracker<T>, H = RandomState> {
  store: Store<T, H>,
  tracker: A,
}

impl<T: Task> Default for Pie<T> {
  #[inline]
  fn default() -> Self { Self::new(NoopTracker::default()) }
}

impl<T: Task, A: Tracker<T> + Default> Pie<T, A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn with_tracker(tracker: A) -> Self { Self { store: Store::default(), tracker } }
}

impl<T: Task, A: Tracker<T> + Default, H: BuildHasher + Default> Pie<T, A, H> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn new(tracker: A) -> Self { Self { store: Store::default(), tracker } }

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
  /// Creates a new [`Pie`] instance with its tracker replaced with `tracker`.
  #[inline]
  pub fn replace_tracker<AA: Tracker<T>>(self, tracker: AA) -> Pie<T, AA, H> {
    let store = self.store;
    Pie { store, tracker }
  }

  /// Serializes the state with the given `serializer`.
  #[cfg(feature = "serde")]
  pub fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> where
    T: serde::Serialize,
    T::Output: serde::Serialize,
  {
    use serde::Serialize;
    self.store.serialize(serializer)
  }
  /// Deserializes the state from the given `deserializer`, and returns a new PIE instance with the deserialized state.
  #[cfg(feature = "serde")]
  pub fn deserialize<'de, D: serde::Deserializer<'de>>(self, deserializer: D) -> Result<Self, D::Error> where
    T: serde::Deserialize<'de>,
    T::Output: serde::Deserialize<'de>,
  {
    use serde::Deserialize;
    let store = Store::deserialize(deserializer)?;
    Ok(Self { store, tracker: self.tracker })
  }
}


/// A session in which builds are executed. Every task is executed at most once each session.
pub struct Session<'p, T: Task, A, H> {
  store: &'p mut Store<T, H>,
  tracker: &'p mut A,
  visited: HashSet<TaskNode, H>,
  dependency_check_errors: Vec<io::Error>,
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
    let mut context = TopDownContext::new(self);
    context.require(task)
  }

  /// Make up-to-date all tasks (transitively) affected by changed files.
  #[inline]
  pub fn update_affected_by<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, changed_files: I) {
    let mut context = BottomUpContext::new(self);
    context.update_affected_by(changed_files);
  }

  /// Gets the [`Tracker`] instance.
  #[inline]
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }
  /// Gets a slice over all errors produced during dependency checks.
  #[inline]
  pub fn dependency_check_errors(&self) -> &[io::Error] { &self.dependency_check_errors }
}
