#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TaskDependency<T, O> {
  pub task: T,
  pub stamper: OutputStamper,
  pub stamp: OutputStamp<O>,
}

impl<T: Task> TaskDependency<T, T::Output> {
  pub fn new(task: T, stamper: OutputStamper, output: T::Output) -> Self {
    let stamp = stamper.stamp(output);
    Self { task, stamper, stamp }
  }

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
