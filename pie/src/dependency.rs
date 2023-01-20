use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;

use crate::{Context, Task};
use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Dependency<T, O> {
  RequireFile(FileDependency),
  ProvideFile(FileDependency),
  RequireTask(TaskDependency<T, O>),
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FileDependency {
  pub path: PathBuf,
  pub stamper: FileStamper,
  pub stamp: FileStamp,
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TaskDependency<T, O> {
  pub task: T,
  pub stamper: OutputStamper,
  pub stamp: OutputStamp<O>,
}

impl<T: Task> Dependency<T, T::Output> {
  #[inline]
  pub fn require_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<(Self, File), std::io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let file = File::open(&path)?;
    let dependency = Self::RequireFile(FileDependency { path, stamper, stamp });
    Ok((dependency, file))
  }
  #[inline]
  pub fn provide_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<Self, std::io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let dependency = Self::ProvideFile(FileDependency { path, stamper, stamp });
    Ok(dependency)
  }
  #[inline]
  pub fn require_task(task: T, output: T::Output, stamper: OutputStamper) -> Self {
    let stamp = stamper.stamp(output);
    Self::RequireTask(TaskDependency { task, stamper, stamp })
  }

  #[inline]
  pub fn is_require_file(&self) -> bool {
    match self {
      Dependency::RequireFile(_) => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_provide_file(&self) -> bool {
    match self {
      Dependency::ProvideFile(_) => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_file_dependency(&self) -> bool {
    match self {
      Dependency::RequireFile(_) => true,
      Dependency::ProvideFile(_) => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_require_task(&self) -> bool {
    match self {
      Dependency::RequireTask(_) => true,
      _ => false,
    }
  }

  #[inline]
  pub fn as_file_dependency(&self) -> Option<&FileDependency> {
    match self {
      Dependency::RequireFile(d) => Some(d),
      Dependency::ProvideFile(d) => Some(d),
      _ => None,
    }
  }
  #[inline]
  pub fn as_task_dependency(&self) -> Option<&TaskDependency<T, T::Output>> {
    match self {
      Dependency::RequireTask(d) => Some(d),
      _ => None,
    }
  }

  #[inline]
  pub fn is_consistent<C: Context<T>>(&self, context: &mut C) -> Result<Option<InconsistentDependency<T::Output>>, Box<dyn Error>> {
    match self {
      Dependency::RequireFile(FileDependency { path, stamper, stamp }) => {
        Self::file_is_consistent(path, stamper, stamp)
          .map(|s| s.map(|s| InconsistentDependency::File(s)))
      }
      Dependency::ProvideFile(FileDependency { path, stamper, stamp }) => {
        Self::file_is_consistent(path, stamper, stamp)
          .map(|s| s.map(|s| InconsistentDependency::File(s)))
      }
      Dependency::RequireTask(TaskDependency { task, stamper, stamp }) => {
        let output = context.require_task(task);
        Ok(Self::task_is_consistent(output, stamper, stamp)
          .map(|s| InconsistentDependency::Task(s)))
      }
    }
  }
  #[inline]
  pub fn require_or_provide_file_is_consistent(&self) -> Result<Option<FileStamp>, Box<dyn Error>> {
    if let Some(FileDependency { path, stamper, stamp }) = self.as_file_dependency() {
      Self::file_is_consistent(path, stamper, stamp)
    } else {
      Ok(None)
    }
  }
  #[inline]
  pub fn require_task_is_consistent_with(&self, output: T::Output) -> Option<OutputStamp<T::Output>> {
    if let Some(TaskDependency { stamper, stamp, .. }) = self.as_task_dependency() {
      Self::task_is_consistent(output, stamper, stamp)
    } else {
      None
    }
  }

  fn file_is_consistent(path: &PathBuf, stamper: &FileStamper, stamp: &FileStamp) -> Result<Option<FileStamp>, Box<dyn Error>> {
    let new_stamp = stamper.stamp(path)?;
    let consistent = new_stamp == *stamp;
    if consistent {
      Ok(None)
    } else {
      Ok(Some(new_stamp))
    }
  }
  fn task_is_consistent(output: T::Output, stamper: &OutputStamper, stamp: &OutputStamp<T::Output>) -> Option<OutputStamp<T::Output>> {
    let new_stamp = stamper.stamp(output);
    let consistent = new_stamp == *stamp;
    if consistent {
      None
    } else {
      Some(new_stamp)
    }
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub enum InconsistentDependency<O> {
  File(FileStamp),
  Task(OutputStamp<O>),
}

impl<O: Debug> InconsistentDependency<O> {
  pub fn unwrap_as_file_stamp(&self) -> &FileStamp {
    match self {
      InconsistentDependency::File(s) => s,
      InconsistentDependency::Task(_) => panic!("attempt to unwrap {:?} as file stamp", self),
    }
  }

  pub fn unwrap_as_output_stamp(&self) -> &OutputStamp<O> {
    match self {
      InconsistentDependency::File(_) => panic!("attempt to unwrap {:?} as output stamp", self),
      InconsistentDependency::Task(s) => s,
    }
  }
}