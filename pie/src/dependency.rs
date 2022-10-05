use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;

use pie_tagged_serde::Id;

use crate::{Context, register_dependency, Task};

/// A dynamic dependency that can be checked for consistency.
pub trait Dependency: Id + Serialize + DeserializeOwned + Debug {
  fn is_consistent<C: Context>(&self, context: &mut C) -> Result<bool, Box<dyn Error>>;
}

/// A dependency to (the output of) another task.
#[derive(Clone, Serialize, Deserialize, Debug)]
pub struct TaskDependency<T, O> {
  task: T,
  output: O,
}

impl<T: Task> TaskDependency<T, T::Output> {
  #[inline]
  pub fn new(task: T, output: T::Output) -> Self { Self { task, output } }
}

impl<T: Task> Dependency for TaskDependency<T, T::Output> {
  #[inline]
  fn is_consistent<C: Context>(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    let output = context.require_task::<T>(&self.task);
    Ok(output == self.output)
  }
}

impl<T: Task> Id for TaskDependency<T, T::Output> {
  fn id() -> &'static str {
    "TaskDependency" // TODO: use generic type argument?
  }
}

/// A dependency to (the contents of) a file.
#[derive(Clone, Serialize, Deserialize, Debug)]
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

impl Dependency for FileDependency {
  #[inline]
  fn is_consistent<C: Context>(&self, _context: &mut C) -> Result<bool, Box<dyn Error>> {
    let modification_date = self.open()?.metadata()?.modified()?;
    Ok(modification_date == self.modification_date)
  }
}

register_dependency!(FileDependency);
