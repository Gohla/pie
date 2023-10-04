use crate::{Context, Task};

pub struct NonIncrementalContext;

impl<T: Task> Context<T> for NonIncrementalContext {
  fn require_task(&mut self, task: &T) -> T::Output {
    task.execute(self)
  }
}

