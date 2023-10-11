

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TaskDependency<T, O> {
  task: T,
  stamper: OutputStamper,
  stamp: OutputStamp<O>,
}

impl<T: Task> TaskDependency<T, T::Output> {
  /// Creates a new `task` dependency with `stamper` and `output`.
  pub fn new(task: T, stamper: OutputStamper, output: T::Output) -> Self {
    let stamp = stamper.stamp(output);
    Self { task, stamper, stamp }
  }

  /// Returns the task of this dependency.
  #[allow(dead_code)]
  pub fn task(&self) -> &T { &self.task }
  /// Returns the stamper of this dependency.
  #[allow(dead_code)]
  pub fn stamper(&self) -> &OutputStamper { &self.stamper }
  /// Returns the stamp of this dependency.
  #[allow(dead_code)]
  pub fn stamp(&self) -> &OutputStamp<T::Output> { &self.stamp }

  /// Checks whether this task dependency is inconsistent, returning:
  /// - `Some(stamp)` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `None` if this dependency is consistent.
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Option<OutputStamp<T::Output>> {
    let output = context.require_task(&self.task);
    let new_stamp = self.stamper.stamp(output);
    if new_stamp == self.stamp {
      None
    } else {
      Some(new_stamp)
    }
  }
}
