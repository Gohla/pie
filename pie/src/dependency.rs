use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;

use crate::{Context, Task};
use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) enum Dependency<T, O> {
  RequireFile(PathBuf, FileStamper, FileStamp),
  ProvideFile(PathBuf, FileStamper, FileStamp),
  RequireTask(T, OutputStamper, OutputStamp<O>),
}

impl<T: Task> Dependency<T, T::Output> {
  pub fn require_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<(Self, File), std::io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let file = File::open(&path)?;
    let dependency = Self::RequireFile(path, stamper, stamp);
    Ok((dependency, file))
  }
  pub fn provide_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<Self, std::io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let dependency = Self::ProvideFile(path, stamper, stamp);
    Ok(dependency)
  }
  pub fn require_task(task: T, output: T::Output, stamper: OutputStamper) -> Self {
    let stamp = stamper.stamp(output);
    Self::RequireTask(task, stamper, stamp)
  }


  pub fn is_require_file(&self) -> bool {
    match self {
      Dependency::RequireFile(_, _, _) => true,
      _ => false,
    }
  }
  pub fn is_provide_file(&self) -> bool {
    match self {
      Dependency::ProvideFile(_, _, _) => true,
      _ => false,
    }
  }
  pub fn is_file_dependency(&self) -> bool {
    match self {
      Dependency::RequireFile(_, _, _) => true,
      Dependency::ProvideFile(_, _, _) => true,
      _ => false,
    }
  }
  pub fn is_task_require(&self) -> bool {
    match self {
      Dependency::RequireTask(_, _, _) => true,
      _ => false,
    }
  }


  pub fn is_consistent<C: Context<T>>(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    match self {
      Dependency::RequireFile(path, stamper, stamp) => Self::file_is_consistent(path, stamper, stamp),
      Dependency::ProvideFile(path, stamper, stamp) => Self::file_is_consistent(path, stamper, stamp),
      Dependency::RequireTask(task, stamper, stamp) => {
        let output = context.require_task(task);
        let new_stamp = stamper.stamp(output);
        Ok(new_stamp == *stamp)
      }
    }
  }
  pub fn require_or_provide_file_is_consistent(&self) -> Result<bool, Box<dyn Error>> {
    match self {
      Dependency::RequireFile(path, stamper, stamp) => {
        Self::file_is_consistent(path, stamper, stamp)
      }
      Dependency::ProvideFile(path, stamper, stamp) => {
        Self::file_is_consistent(path, stamper, stamp)
      }
      _ => Ok(false),
    }
  }
  pub fn require_task_is_consistent_with(&self, output: T::Output) -> bool {
    match self {
      Dependency::RequireTask(_, stamper, stamp) => {
        let new_stamp = stamper.stamp(output);
        new_stamp == *stamp
      }
      _ => false,
    }
  }

  fn file_is_consistent(path: &PathBuf, stamper: &FileStamper, stamp: &FileStamp) -> Result<bool, Box<dyn Error>> {
    let new_stamp = stamper.stamp(path)?;
    Ok(new_stamp == *stamp)
  }
}
