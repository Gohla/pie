use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};

use crate::{Context, Task};

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum Dependency<T, O> {
  RequireFile(PathBuf, SystemTime),
  ProvideFile(PathBuf, SystemTime),
  RequireTask(T, O),
}

impl<T: Task> Dependency<T, T::Output> {
  pub fn require_file(path: impl Into<PathBuf>) -> Result<(Self, File), std::io::Error> {
    let path = path.into();
    let file = File::open(&path)?;
    let modification_date = file.metadata()?.modified()?;
    let dependency = Self::RequireFile(path, modification_date);
    Ok((dependency, file))
  }

  pub fn provide_file(path: impl Into<PathBuf>) -> Result<Self, std::io::Error> {
    let path = path.into();
    let file = File::open(&path)?;
    let modification_date = file.metadata()?.modified()?;
    let dependency = Self::ProvideFile(path, modification_date);
    Ok(dependency)
  }

  pub fn require_task(task: T, output: T::Output) -> Self {
    Self::RequireTask(task, output)
  }

  pub fn is_consistent<C: Context<T>>(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    match self {
      Dependency::RequireFile(path, modification_date) => Self::file_is_consistent(path, modification_date),
      Dependency::ProvideFile(path, modification_date) => Self::file_is_consistent(path, modification_date),
      Dependency::RequireTask(task, output) => Ok(context.require_task(task) == *output),
    }
  }

  fn file_is_consistent(path: &PathBuf, modification_date: &SystemTime) -> Result<bool, Box<dyn Error>> {
    let new_modification_date = File::open(path)?.metadata()?.modified()?;
    Ok(new_modification_date == *modification_date)
  }
}
