use std::fmt::Debug;
use std::fs::File;
use std::hash::Hash;
use std::io;
use std::path::Path;

pub mod context;

pub trait Task: Clone + Eq + Hash + Debug {
  type Output: Clone + Eq + Debug;
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output;
}

pub trait Context<T: Task> {
  fn require_task(&mut self, task: &T) -> T::Output;
  fn require_file<P: AsRef<Path>>(&mut self, path: P) -> Result<Option<File>, io::Error>;
}
