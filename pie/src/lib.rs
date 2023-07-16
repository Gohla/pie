//! PIE is a *sound* and *incremental* *programmatic build system*, a mix between an *incremental build system* and 
//! *incremental computation* system that can be implemented and used *programmatically*.
//!
//! # Tasks
//!
//! [Tasks](Task) are the unit of computation in a programmatic incremental build system. They have an 
//! [execute](Task::execute) function that executes the task and returns its result. Therefore, tasks can be seen as a 
//! sort of closure, an executable function along with its input, but one that can be executed incrementally by PIE.
//!
//! Tasks *dynamically create dependencies while the build is running* through methods on [`Context`], enabling precise
//! and expressive dependency tracking.
//!
//! The identity of a task is determined by its [`Eq`] and [`Hash`] implementations. Therefore, tasks that are 
//! structurally different are considered different.
//!
//! # Soundness and Incrementality through Consistency Checking of Dynamic Dependencies
//!
//! PIE is sound and incremental, because it executes a task if and only if it is *inconsistent*.
//!
//! A task is *new* if it has *not been executed before*. New tasks are *always inconsistent*. If it is not new, it is 
//! an already *existing task*. An existing task is *checked* for consistency. An existing task is inconsistent if 
//! and only if any of its dependencies are inconsistent:
//!
//! - A file dependency is inconsistent if its [file stamp](stamp::FileStamp) changes. 
//! - A task dependency is inconsistent if, after *recursively checking* the task, its 
//!   [output stamp](stamp::OutputStamp) changes.
//!
//! Dependencies store the stamp that was created when the dependency was created. The stamp of a dependency changes when
//! the new stamp differs from the stored stamp, using the [`Eq`] implementation of the corresponding stamp.
//!
//! If all dependencies of an existing task are consistent after checking, or if the existing task has no dependencies, 
//! it is *consistent* and is not executed (at that time). The recursive nature of checking task dependencies ensures 
//! that indirect changes can cause tasks to become inconsistent, and cause them to be correctly executed, even in the
//! presence of dynamic dependencies.
//!
//! # Creating Correct Dependencies
//!
//! It is up to the task author to correctly create all dependencies required for a task to be considered inconsistent.
//! Creating the wrong dependency, or forgetting to create a dependency, can cause incrementality bugs or decreased 
//! incrementality, even if PIE is sound and incremental.
//!
//! # Build Sessions
//!
//! [Build sessions](Session) place restrictions on whether a task is checked or executed, to make builds incremental 
//! and sound in the face of filesystem changes. The following two restrictions are in place:
//!
//! - A task is *checked at most once each session*.
//! - A task is *executed at most once each session*.
//!
//! Therefore, once a task is checked (and deemed consistent), or once a task is executed, it will not be checked or 
//! executed again this session, even though one of its (file) dependencies may become inconsistent later.
//!
//! The result of this is that *changes made to source files during a session* are *not guaranteed to be detected*. For 
//! example, if a file dependency is consistent when the task is checked, but later becomes inconsistent, we do not 
//! guarantee that the task will be executed.
//!
//! Therefore, a new session must be created in order to re-check or re-execute tasks that are consistent.

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

/// The unit of computation in a programmatic incremental build system. See the [module-level documentation](index.html)
/// for more information.
pub trait Task: Clone + Eq + Hash + Debug {
  /// Type of output this task returns when executed.
  type Output: Clone + Eq + Debug;
  /// Execute the task, using `context` to specify dynamic dependencies, returning `Self::Output`.
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}


/// Programmatic incremental build context, enabling tasks to create dynamic dependencies that context implementations 
/// use for incremental execution. See the [module-level documentation](index.html) for more information.
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


/// Main entry point into PIE, a sound and incremental programmatic build system. See the 
/// [module-level documentation](index.html) for more information.
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

/// A session in which builds are executed. See the [module-level documentation](index.html) for more information.
pub struct Session<'p, T, O, A, H> {
  store: &'p mut Store<T, O, H>,
  tracker: &'p mut A,
  current_executing_task: Option<TaskNode>,
  consistent: HashSet<TaskNode, H>,
  dependency_check_errors: Vec<io::Error>,
}

impl<'p, T: Task, A: Tracker<T>, H: BuildHasher + Default> Session<'p, T, T::Output, A, H> {
  #[inline]
  fn new(pie: &'p mut Pie<T, T::Output, A, H>) -> Self {
    Self {
      store: &mut pie.store,
      tracker: &mut pie.tracker,
      current_executing_task: None,
      consistent: HashSet::default(),
      dependency_check_errors: Vec::default(),
    }
  }

  /// Requires given `task`, returning its up-to-date output.
  #[inline]
  pub fn require(&mut self, task: &T) -> T::Output {
    self.current_executing_task = None;

    let mut context = TopDownContext::new(self);
    context.require_initial(task)
  }

  /// Make up-to-date all tasks (transitively) affected by `changed_files`.
  #[inline]
  pub fn update_affected_by<'a, I: IntoIterator<Item=&'a PathBuf> + Clone>(&mut self, changed_files: I) {
    self.current_executing_task = None;

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
