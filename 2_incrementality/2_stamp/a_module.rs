use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::path::Path;

pub mod stamp;
pub mod context;
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
  /// Requires file at given `path`, recording a dependency to it. Call this method *just before reading from the file*,
  /// so that the dependency corresponds to the data that you are reading. Returns:
  /// - `Ok(Some(file))` if a file exists at given `path`,
  /// - `Ok(None)` if no file exists at given `path` (but a directory could exist at given `path`),
  /// - `Err(e)` if there was an error getting the metadata for given `path`, or if there was an error opening the file.
  fn require_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>, io::Error>;

  /// Requires given `task`, recording a dependency and selectively executing it. Returns its up-to-date output.
  fn require_task(&mut self, task: &T) -> T::Output;
}
