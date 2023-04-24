#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct TaskDependency<T, O> {
  pub task: T,
  pub output: O,
}

impl<T: Task> TaskDependency<T, T::Output> {
  pub fn new(task: T, output: T::Output) -> Self {
    Self { task, output }
  }

  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> bool {
    let output = context.require_task(&self.task);
    output != self.output
  }
}
