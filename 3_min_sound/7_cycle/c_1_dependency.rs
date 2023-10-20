use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::Task;
use crate::fs::open_if_file;
use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct FileDependency {
  path: PathBuf,
  stamper: FileStamper,
  stamp: FileStamp,
}

impl FileDependency {
  /// Creates a new file dependency with `path` and `stamper`, returning:
  /// - `Ok(file_dependency)` normally,
  /// - `Err(e)` if stamping failed.
  #[allow(dead_code)]
  pub fn new(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<Self, io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let dependency = FileDependency { path, stamper, stamp };
    Ok(dependency)
  }
  /// Creates a new file dependency with `path` and `stamper`, returning:
  /// - `Ok((file_dependency, Some(file)))` if a file exists at given path,
  /// - `Ok((file_dependency, None))` if no file exists at given path (but a directory could exist at given path),
  /// - `Err(e)` if stamping or opening the file failed.
  pub fn new_with_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<(Self, Option<File>), io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let file = open_if_file(&path)?;
    let dependency = FileDependency { path, stamper, stamp };
    Ok((dependency, file))
  }

  /// Returns the path of this dependency.
  #[allow(dead_code)]
  pub fn path(&self) -> &PathBuf { &self.path }
  /// Returns the stamper of this dependency.
  #[allow(dead_code)]
  pub fn stamper(&self) -> &FileStamper { &self.stamper }
  /// Returns the stamp of this dependency.
  #[allow(dead_code)]
  pub fn stamp(&self) -> &FileStamp { &self.stamp }

  /// Checks whether this file dependency is inconsistent, returning:
  /// - `Ok(Some(stamp))` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `Ok(None)` if this dependency is consistent,
  /// - `Err(e)` if there was an error checking this dependency for consistency.
  pub fn is_inconsistent(&self) -> Result<Option<FileStamp>, io::Error> {
    let new_stamp = self.stamper.stamp(&self.path)?;
    if new_stamp == self.stamp {
      Ok(None)
    } else {
      Ok(Some(new_stamp))
    }
  }
}


#[derive(Clone, Eq, PartialEq, Debug)]
pub struct TaskDependency<T, O> {
  task: T,
  stamper: OutputStamper,
  stamp: OutputStamp<O>,
}

impl<T: Task> TaskDependency<T, T::Output> {
  /// Creates a new `task` dependency with `stamper` and `output`.
  pub fn new(task: T, stamper: OutputStamper, output: T::Output) -> Self {
    let stamp = stamper.stamp(output);
    Self { task, stamper, stamp }
  }

  /// Returns the task of this dependency.
  #[allow(dead_code)]
  pub fn task(&self) -> &T { &self.task }
  /// Returns the stamper of this dependency.
  #[allow(dead_code)]
  pub fn stamper(&self) -> &OutputStamper { &self.stamper }
  /// Returns the stamp of this dependency.
  #[allow(dead_code)]
  pub fn stamp(&self) -> &OutputStamp<T::Output> { &self.stamp }

  /// Checks whether this task dependency is inconsistent, returning:
  /// - `Some(stamp)` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `None` if this dependency is consistent.
  pub fn is_inconsistent<C: MakeConsistent<T>>(&self, context: &mut C) -> Option<OutputStamp<T::Output>> {
    let output = context.make_task_consistent(&self.task);
    let new_stamp = self.stamper.stamp(output);
    if new_stamp == self.stamp {
      None
    } else {
      Some(new_stamp)
    }
  }
}

/// Make a task consistent without adding dependencies.
pub trait MakeConsistent<T: Task> {
  fn make_task_consistent(&mut self, task: &T) -> T::Output;
}


#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Dependency<T, O> {
  RequireFile(FileDependency),
  ProvideFile(FileDependency),
  RequireTask(TaskDependency<T, O>),
  ReservedRequireTask,
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Inconsistency<O> {
  File(FileStamp),
  Task(OutputStamp<O>),
}

impl<T: Task> Dependency<T, T::Output> {
  /// Checks whether this dependency is inconsistent, returning:
  /// - `Ok(Some(stamp))` if the dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `Ok(None)` if the dependency is consistent,
  /// - `Err(e)` if there was an error checking the dependency for consistency.
  ///
  /// # Panics
  ///
  /// Panics when this dependency is a [Dependency::ReservedRequireTask] dependency.
  pub fn is_inconsistent<C: MakeConsistent<T>>(&self, context: &mut C) -> Result<Option<Inconsistency<T::Output>>, io::Error> {
    let option = match self {
      Dependency::RequireFile(d) => d.is_inconsistent()?
        .map(|s| Inconsistency::File(s)),
      Dependency::ProvideFile(d) => d.is_inconsistent()?
        .map(|s| Inconsistency::File(s)),
      Dependency::RequireTask(d) => d.is_inconsistent(context)
        .map(|s| Inconsistency::Task(s)),
      Dependency::ReservedRequireTask => panic!("BUG: consistency checking reserved task dependency"),
    };
    Ok(option)
  }
}


#[cfg(test)]
mod test {
  use std::fs::write;
  use std::io::{self, Read};

  use dev_shared::{create_temp_file, write_until_modified};

  use crate::Context;
  use crate::context::non_incremental::NonIncrementalContext;

  use super::*;

  /// Task that reads file at given path and returns it contents as a string.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  struct ReadStringFromFile(PathBuf);

  impl Task for ReadStringFromFile {
    type Output = String;
    fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
      let mut string = String::new();
      let file = context.require_file(&self.0).expect("failed to require file");
      if let Some(mut file) = file {
        file.read_to_string(&mut string).expect("failed to read from file");
      };
      string
    }
  }

  #[test]
  fn test_file_dependency_consistency() -> Result<(), io::Error> {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file()?;
    write(&temp_file, "test1")?;

    let file_dependency = FileDependency::new(temp_file.path(), FileStamper::Modified)?;
    let require_dependency: Dependency<ReadStringFromFile, String> = Dependency::RequireFile(file_dependency.clone());
    let provide_dependency: Dependency<ReadStringFromFile, String> = Dependency::ProvideFile(file_dependency.clone());
    assert!(file_dependency.is_inconsistent()?.is_none());
    assert!(require_dependency.is_inconsistent(&mut context)?.is_none());
    assert!(provide_dependency.is_inconsistent(&mut context)?.is_none());

    // Change the file, changing the stamp the stamper will create next time, making the file dependency inconsistent.
    write_until_modified(&temp_file, "test2")?;
    assert!(file_dependency.is_inconsistent()?.is_some());
    assert!(require_dependency.is_inconsistent(&mut context)?.is_some());
    assert!(provide_dependency.is_inconsistent(&mut context)?.is_some());

    Ok(())
  }

  #[test]
  fn test_task_dependency_consistency() -> Result<(), io::Error> {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file()?;
    write(&temp_file, "test1")?;
    let task = ReadStringFromFile(temp_file.path().to_path_buf());
    let output = context.require_task(&task);

    let task_dependency = TaskDependency::new(task.clone(), OutputStamper::Equals, output);
    let dependency = Dependency::RequireTask(task_dependency.clone());
    assert!(task_dependency.is_inconsistent(&mut context).is_none());
    assert!(dependency.is_inconsistent(&mut context)?.is_none());

    // Change the file, causing the task to return a different output, changing the stamp the stamper will create next
    // time, making the task dependency inconsistent.
    write_until_modified(&temp_file, "test2")?;
    assert!(task_dependency.is_inconsistent(&mut context).is_some());
    assert!(dependency.is_inconsistent(&mut context)?.is_some());

    Ok(())
  }
}
