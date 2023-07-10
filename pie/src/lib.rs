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

/// A unit of computation in a programmatic incremental build system.
pub trait Task: Clone + Eq + Hash + Debug {
  /// Type of output this task returns when executed.
  type Output: Clone + Eq + Debug;
  /// Execute the task, using `context` to specify dynamic dependencies, returning `Self::Output`.
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}

/// Programmatic incremental build context, enabling tasks to create dynamic dependencies that context implementations 
/// use for incremental execution.
pub trait Context<T: Task> {
  /// Requires file at given `path`, recording a read-dependency to it (using the default require file stamper). Call 
  /// this method *just before reading from the file*, so that the dependency corresponds to the data that you are 
  /// reading. Returns:
  /// - `Ok(Some(file))` if a file exists at given `path` with `file` in read-only mode, 
  /// - `Ok(None)` if no file exists at given `path` (but a directory could exist at given `path`),
  /// - `Err(e)` if there was an error getting the metadata for given `path`, if there was an error opening the file, or 
  ///   if there was an error stamping the file.
  #[inline]
  fn require_file(&mut self, path: impl AsRef<Path>) -> Result<Option<File>, io::Error> {
    self.require_file_with_stamper(path, self.default_require_file_stamper())
  }
  /// Requires file at given `path`, recording a read-dependency to it (using given `stamper`). Call this method 
  /// *just before reading from the file*, so that the dependency corresponds to the data that you are reading. Returns:
  /// - `Ok(Some(file))` if a file exists at given `path` with `file` in read-only mode, 
  /// - `Ok(None)` if no file exists at given `path` (but a directory could exist at given `path`),
  /// - `Err(e)` if there was an error getting the metadata for given `path`, if there was an error opening the file, or 
  ///   if there was an error stamping the file.
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error>;
  /// Returns the default require file stamper.
  #[inline]
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }

  /// Provides file at given `path`, recording a write-dependency to it (using the default provide file stamper) . Call 
  /// this method *just after writing to the file*, so that the dependency corresponds to the data that you wrote. 
  /// This method does not return the opened file, as it must be called *after writing to the file*.
  ///
  /// # Errors
  ///
  /// If stamping the file fails, returns that error.
  #[inline]
  fn provide_file(&mut self, path: impl AsRef<Path>) -> Result<(), io::Error> {
    self.provide_file_with_stamper(path, self.default_provide_file_stamper())
  }
  /// Provides file at given `path`, recording a write-dependency to it (using given `stamper`). Call this method 
  /// *just after writing to the file*, so that the dependency corresponds to the data that you wrote. 
  /// This method does not return the opened file, as it must be called *after writing to the file*.
  ///
  /// # Errors
  ///
  /// If stamping the file fails, returns that error.
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), io::Error>;
  /// Returns the default provide file stamper.
  #[inline]
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }

  /// Requires given `task`, recording a dependency (using the default output stamper) and selectively executing it. 
  /// Returns its up-to-date output.
  #[inline]
  fn require_task(&mut self, task: &T) -> T::Output {
    self.require_task_with_stamper(task, self.default_output_stamper())
  }
  /// Requires given `task`, recording a dependency (using given `stamper`) and selectively executing it. Returns its
  /// up-to-date output.
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output;
  /// Returns the default output stamper.
  #[inline]
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
}


/// Main entry point into the PIE build system.
pub struct Pie<T, O, A = NoopTracker<T>, H = RandomState> {
  store: Store<T, O, H>,
  tracker: A,
}

impl<T: Task> Default for Pie<T, T::Output> {
  #[inline]
  fn default() -> Self { Self::new(NoopTracker::default()) }
}

impl<T: Task, A: Tracker<T>> Pie<T, T::Output, A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn with_tracker(tracker: A) -> Self { Self { store: Store::default(), tracker } }
}

impl<T: Task, A: Tracker<T> + Default, H: BuildHasher + Default> Pie<T, T::Output, A, H> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  #[inline]
  pub fn new(tracker: A) -> Self { Self { store: Store::new(), tracker } }

  /// Creates a new build session. Only one session may be active at once, enforced via mutable (exclusive) borrow.
  #[inline]
  pub fn new_session(&mut self) -> Session<T, T::Output, A, H> { Session::new(self) }
  /// Runs `f` inside a new session.
  #[inline]
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session<T, T::Output, A, H>) -> R) -> R {
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
  pub fn replace_tracker<AA: Tracker<T>>(self, tracker: AA) -> Pie<T, T::Output, AA, H> {
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


/// A session in which builds are executed. A task is executed at most once each session.
pub struct Session<'p, T, O, A, H> {
  store: &'p mut Store<T, O, H>,
  tracker: &'p mut A,
  current_executing_task: Option<TaskNode>,
  visited: HashSet<TaskNode, H>,
  dependency_check_errors: Vec<io::Error>,
}

impl<'p, T: Task, A: Tracker<T>, H: BuildHasher + Default> Session<'p, T, T::Output, A, H> {
  #[inline]
  fn new(pie: &'p mut Pie<T, T::Output, A, H>) -> Self {
    Self {
      store: &mut pie.store,
      tracker: &mut pie.tracker,
      current_executing_task: None,
      visited: HashSet::default(),
      dependency_check_errors: Vec::default(),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require(&mut self, task: &T) -> T::Output {
    let mut context = TopDownContext::new(self);
    context.require_initial(task)
  }

  /// Make up-to-date all tasks (transitively) affected by `changed_files`.
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
