use std::fmt::Debug;
use std::hash::Hash;

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
  /// Requires given `task`, recording a dependency and selectively executing it. Returns its up-to-date output.
  fn require_task(&mut self, task: &T) -> T::Output;
}
