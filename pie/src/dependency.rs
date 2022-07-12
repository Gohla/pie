use std::error::Error;
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::{Context, Task};

/// A dynamic dependency that can be checked for consistency.
pub trait Dependency<C: Context> {
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>>;
}


/// A dependency to (the output of) another task.
#[derive(Clone)]
pub struct TaskDependency<T: Task> {
  task: T,
  output: T::Output,
}

impl<T: Task> TaskDependency<T> {
  #[inline]
  pub fn new(task: T, output: T::Output) -> Self { Self { task, output } }
}

impl<T: Task, C: Context> Dependency<C> for TaskDependency<T> {
  #[inline]
  fn is_consistent(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    let output = context.require_task::<T>(&self.task);
    Ok(output == self.output)
  }
}


/// A dependency to (the contents of) a file.
#[derive(Clone)]
pub struct FileDependency {
  path: PathBuf,
  modification_date: SystemTime,
}

impl FileDependency {
  #[inline]
  pub fn new(path: PathBuf) -> Result<Self, std::io::Error> {
    let modification_date = File::open(&path)?.metadata()?.modified()?;
    Ok(Self { path, modification_date })
  }
  #[inline]
  pub fn open(&self) -> Result<File, std::io::Error> { File::open(&self.path) }
}

impl<C: Context> Dependency<C> for FileDependency {
  #[inline]
  fn is_consistent(&self, _context: &mut C) -> Result<bool, Box<dyn Error>> {
    let modification_date = self.open()?.metadata()?.modified()?;
    Ok(modification_date == self.modification_date)
  }
}
