use std::fmt::Debug;
use std::hash::Hash;

pub trait Task: Clone + Eq + Hash + Debug {
  type Output: Clone + Eq + Debug;
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}

pub trait Context<T: Task> {
  fn require_task(&mut self, task: &T) -> T::Output;
}
