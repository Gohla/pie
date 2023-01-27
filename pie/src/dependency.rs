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
  pub fn as_require_or_provide_file_dependency(&self, provide: bool) -> Option<&FileDependency> {
    match self {
      Dependency::RequireFile(d) => Some(d),
      Dependency::ProvideFile(d) => provide.then_some(d),
      _ => None,
    }
  }
  #[inline]
  pub fn as_require_file_dependency(&self) -> Option<&FileDependency> {
    match self {
      Dependency::RequireFile(d) => Some(d),
      _ => None,
    }
  }
  #[inline]
  pub fn as_provide_file_dependency(&self) -> Option<&FileDependency> {
    match self {
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

  /// Checks whether this dependency is inconsistent, returning:
  /// - `Ok(Some(stamp))` if the dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `Ok(None)` if the dependency is consistent,
  /// - `Err(e)` if there was an error checking the dependency for consistency.
  #[inline]
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Result<Option<InconsistentDependency<T::Output>>, Box<dyn Error>> {
    match self {
      Dependency::RequireFile(d) => {
        d.is_inconsistent()
          .map(|s| s.map(|s| InconsistentDependency::File(s)))
      }
      Dependency::ProvideFile(d) => {
        d.is_inconsistent()
          .map(|s| s.map(|s| InconsistentDependency::File(s)))
      }
      Dependency::RequireTask(d) => {
        Ok(d.is_inconsistent(context)
          .map(|s| InconsistentDependency::Task(s)))
      }
    }
  }
}

impl<T: Task> TaskDependency<T, T::Output> {
  /// Checks whether this task dependency is inconsistent, returning:
  /// - `Some(stamp)` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `None` if this dependency is consistent.
  #[inline]
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Option<OutputStamp<T::Output>> {
    let output = context.require_task(&self.task);
    self.is_inconsistent_with(output)
  }
  /// Checks whether this task dependency is inconsistent with given `output`, returning:
  /// - `Some(stamp)` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `None` if this dependency is consistent.
  #[inline]
  pub fn is_inconsistent_with(&self, output: T::Output) -> Option<OutputStamp<T::Output>> {
    Self::task_is_consistent(output, &self.stamper, &self.stamp)
  }

  #[inline]
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

impl FileDependency {
  /// Checks whether this file dependency is inconsistent, returning:
  /// - `Ok(Some(stamp))` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `Ok(None)` if this dependency is consistent,
  /// - `Err(e)` if there was an error checking this dependency for consistency.
  #[inline]
  pub fn is_inconsistent(&self) -> Result<Option<FileStamp>, Box<dyn Error>> {
    let new_stamp = self.stamper.stamp(&self.path)?;
    let consistent = new_stamp == self.stamp;
    if consistent {
      Ok(None)
    } else {
      Ok(Some(new_stamp))
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
