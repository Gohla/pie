use std::collections::HashSet;
use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::path::Path;

use stamp::{FileStamper, OutputStamper};

use crate::context::top_down::TopDownContext;
use crate::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};

pub mod stamp;
pub mod dependency;
pub mod tracker;
mod context;
mod fs;
mod store;

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
  /// Requires file at given `path`, recording a dependency to it (using the default require file stamper). Call this
  /// method *just before reading from the file*, so that the dependency corresponds to the data that you are reading.
  /// Returns:
  /// - `Ok(Some(file))` if a file exists at given `path`,
  /// - `Ok(None)` if no file exists at given `path` (but a directory could exist at given `path`),
  /// - `Err(e)` if there was an error getting the metadata for given `path`, if there was an error opening the file, or
  ///   if there was an error stamping the file.
  fn require_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>, io::Error> {
    self.require_file_with_stamper(path, self.default_require_file_stamper())
  }
  /// Requires file at given `path`, recording a dependency to it (using given `stamper`). Call this method
  /// *just before reading from the file*, so that the dependency corresponds to the data that you are reading. Returns:
  /// - `Ok(Some(file))` if a file exists at given `path`,
  /// - `Ok(None)` if no file exists at given `path` (but a directory could exist at given `path`),
  /// - `Err(e)` if there was an error getting the metadata for given `path`, if there was an error opening the file, or
  ///   if there was an error stamping the file.
  fn require_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<Option<File>, io::Error>;
  /// Returns the default require file stamper.
  fn default_require_file_stamper(&self) -> FileStamper { FileStamper::Modified }

  /// Provides file at given `path`, recording a dependency to it (using the default provide file stamper). Call this
  /// method *just after writing to the file*, so that the dependency corresponds to your written data. Returns an
  /// `Err(e)` if there was an error getting the metadata for given `path`, or if there was an error stamping the file.
  fn provide_file<P: AsRef<Path>>(&mut self, path: P) -> Result<(), io::Error> {
    self.provide_file_with_stamper(path, self.default_provide_file_stamper())
  }
  /// Provides file at given `path`, recording a dependency to it (using given `stamper`). Call this method
  /// *just after writing to the file*, so that the dependency corresponds to you written data.  Returns an `Err(e)` if
  /// there was an error getting the metadata for given `path`, or if there was an error stamping the file.
  fn provide_file_with_stamper<P: AsRef<Path>>(&mut self, path: P, stamper: FileStamper) -> Result<(), io::Error>;
  /// Returns the default provide file stamper.
  fn default_provide_file_stamper(&self) -> FileStamper { FileStamper::Modified }

  /// Requires given `task`, recording a dependency (using the default output stamper) and selectively executing it.
  /// Returns its up-to-date output.
  fn require_task(&mut self, task: &T) -> T::Output {
    self.require_task_with_stamper(task, self.default_output_stamper())
  }
  /// Requires given `task`, recording a dependency (using given `stamper`) and selectively executing it. Returns its
  /// up-to-date output.
  fn require_task_with_stamper(&mut self, task: &T, stamper: OutputStamper) -> T::Output;
  /// Returns the default output stamper.
  fn default_output_stamper(&self) -> OutputStamper { OutputStamper::Equals }
}

/// Main entry point into PIE, a sound and incremental programmatic build system.
pub struct Pie<T, O, A = NoopTracker> {
  store: Store<T, O>,
  tracker: A,
}

impl<T: Task> Default for Pie<T, T::Output> {
  fn default() -> Self { Self::with_tracker(NoopTracker) }
}

impl<T: Task, A: Tracker<T>> Pie<T, T::Output, A> {
  /// Creates a new [`Pie`] instance with given `tracker`.
  pub fn with_tracker(tracker: A) -> Self { Self { store: Store::default(), tracker } }

  /// Creates a new build session. Only one session may be active at once, enforced via mutable (exclusive) borrow.
  pub fn new_session(&mut self) -> Session<T, T::Output, A> { Session::new(self) }
  /// Runs `f` inside a new build session.
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session<T, T::Output, A>) -> R) -> R {
    let session = self.new_session();
    f(session)
  }

  /// Gets the [`Tracker`] instance.
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }
}

/// A session in which builds are executed.
pub struct Session<'p, T, O, A> {
  store: &'p mut Store<T, O>,
  tracker: &'p mut A,
  current_executing_task: Option<TaskNode>,
  consistent: HashSet<TaskNode>,
  dependency_check_errors: Vec<io::Error>,
}

impl<'p, T: Task, A: Tracker<T>> Session<'p, T, T::Output, A> {
  fn new(pie: &'p mut Pie<T, T::Output, A>) -> Self {
    Self {
      store: &mut pie.store,
      tracker: &mut pie.tracker,
      current_executing_task: None,
      consistent: HashSet::default(),
      dependency_check_errors: Vec::default(),
    }
  }

  /// Requires `task`, returning its up-to-date output.
  pub fn require(&mut self, task: &T) -> T::Output {
    self.current_executing_task = None;
    TopDownContext::new(self).require_initial(task)
  }

  /// Gets the [`Tracker`] instance.
  pub fn tracker(&self) -> &A { &self.tracker }
  /// Gets the mutable [`Tracker`] instance.
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }

  /// Gets all errors produced during dependency checks.
  pub fn dependency_check_errors(&self) -> &[io::Error] { &self.dependency_check_errors }
}
