use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::{Context, Task};
use crate::fs::open_if_file;
use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};

#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct FileDependency {
  path: PathBuf,
  stamper: FileStamper,
  stamp: FileStamp,
}

impl FileDependency {
  /// Creates a new file dependency with `path` and `stamper`, returning:
  /// - `Ok(file_dependency)` normally,
  /// - `Err(e)` if stamping failed.
  #[inline]
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
  #[inline]
  pub fn new_with_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<(Self, Option<File>), io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let file = open_if_file(&path)?;
    let dependency = FileDependency { path, stamper, stamp };
    Ok((dependency, file))
  }

  #[inline]
  pub fn path(&self) -> &PathBuf { &self.path }
  #[inline]
  pub fn stamper(&self) -> &FileStamper { &self.stamper }
  #[inline]
  pub fn stamp(&self) -> &FileStamp { &self.stamp }

  /// Checks whether this file dependency is inconsistent, returning:
  /// - `Ok(Some(stamp))` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `Ok(None)` if this dependency is consistent,
  /// - `Err(e)` if there was an error checking this dependency for consistency.
  #[inline]
  pub fn is_inconsistent(&self) -> Result<Option<FileStamp>, io::Error> {
    let new_stamp = self.stamper.stamp(&self.path)?;
    let consistent = new_stamp == self.stamp;
    if consistent {
      Ok(None)
    } else {
      Ok(Some(new_stamp))
    }
  }
}


#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct TaskDependency<T, O> {
  task: T,
  stamper: OutputStamper,
  stamp: Option<OutputStamp<O>>,
}

impl<T: Task> TaskDependency<T, T::Output> {
  /// Creates a new `task` dependency using `stamper` on `output` to create a stamp.
  #[inline]
  pub fn new(task: T, stamper: OutputStamper, output: T::Output) -> Self {
    let stamp = Some(stamper.stamp(output));
    Self { task, stamper, stamp }
  }

  /// Creates a new reserved `task` dependency with `stamper`. A reserved task dependency does not have an output yet, 
  /// so no stamp can be created, thus its stamp is set to `None`.
  #[inline]
  pub fn new_reserved(task: T, stamper: OutputStamper) -> Self {
    Self { task, stamper, stamp: None }
  }
  /// Updates a reserved task dependency with `output`, storing the stamp created from that output. The task dependency
  /// is not reserved any more after this operation.
  ///
  /// # Panics
  ///
  /// Panics if this is not reserved task dependency (i.e., `self.stamp` is `Some`)
  #[inline]
  pub fn update_reserved(&mut self, output: T::Output) {
    if self.stamp.is_some() {
      panic!("BUG: attempt to update non-reserved task dependency: {:?}", self.task);
    }
    self.stamp = Some(self.stamper.stamp(output));
  }
  
  #[inline]
  pub fn task(&self) -> &T { &self.task }
  #[inline]
  pub fn stamper(&self) -> &OutputStamper { &self.stamper }
  #[inline]
  pub fn stamp(&self) -> Option<&OutputStamp<T::Output>> { self.stamp.as_ref() }

  /// Checks whether this task dependency is inconsistent, returning:
  /// - `Some(stamp)` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `None` if this dependency is consistent.
  ///
  /// # Panics
  ///
  /// Panics if this is a reserved task dependency (i.e., `self.stamp` is `None`)
  #[inline]
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Option<OutputStamp<T::Output>> {
    let Some(stamp) = &self.stamp else {
      panic!("BUG: attempt to consistency check reserved task dependency: {:?}", self.task);
    };
    let output = context.require_task(&self.task);
    let new_stamp = self.stamper.stamp(output);
    let consistent = new_stamp == *stamp;
    if consistent {
      None
    } else {
      Some(new_stamp)
    }
  }
  /// Checks whether this task dependency is inconsistent with given `output`, returning:
  /// - `Some(stamp)` if this dependency is inconsistent (with `stamp` being the new stamp of the dependency),
  /// - `None` if this dependency is consistent.
  ///
  /// # Panics
  ///
  /// Panics if this is a reserved task dependency (i.e., `self.stamp` is `None`)
  #[inline]
  pub fn is_inconsistent_with<'a>(&self, output: &'a T::Output) -> Option<OutputStamp<&'a T::Output>> {
    let Some(stamp) = &self.stamp else {
      panic!("BUG: attempt to consistency check reserved task dependency: {:?}", self.task);
    };
    let new_stamp = self.stamper.stamp(output);
    let consistent = new_stamp == stamp.as_ref();
    if consistent {
      None
    } else {
      Some(new_stamp)
    }
  }
}


#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum Dependency<T, O> {
  RequireFile(FileDependency),
  ProvideFile(FileDependency),
  RequireTask(TaskDependency<T, O>),
}

#[derive(Clone, Eq, PartialEq, Debug)]
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

impl<T: Task> Dependency<T, T::Output> {
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
  pub fn is_file(&self) -> bool {
    match self {
      Dependency::RequireFile(_) => true,
      Dependency::ProvideFile(_) => true,
      _ => false,
    }
  }
  #[inline]
  pub fn is_task(&self) -> bool {
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
  ///
  /// # Panics
  ///
  /// Panics if this is a reserved task dependency (i.e., `self.stamp` is `None`)
  #[inline]
  pub fn is_inconsistent<C: Context<T>>(&self, context: &mut C) -> Result<Option<InconsistentDependency<T::Output>>, io::Error> {
    let option = match self {
      Dependency::RequireFile(d) => {
        d.is_inconsistent()?
          .map(|s| InconsistentDependency::File(s))
      }
      Dependency::ProvideFile(d) => {
        d.is_inconsistent()?
          .map(|s| InconsistentDependency::File(s))
      }
      Dependency::RequireTask(d) => {
        d.is_inconsistent(context)
          .map(|s| InconsistentDependency::Task(s))
      }
    };
    Ok(option)
  }
}


#[cfg(test)]
mod test {
  use std::fs;
  use std::io::Read;
  use std::path::Path;

  use dev_shared::fs::create_temp_file;

  use crate::context::non_incremental::NonIncrementalContext;

  use super::*;

  /// Task that reads file at given path and returns it contents as a string.
  #[derive(Clone, PartialEq, Eq, Hash, Debug)]
  pub struct ReadStringFromFile(PathBuf);

  impl ReadStringFromFile {
    pub fn new(path: impl AsRef<Path>) -> Self { Self(path.as_ref().to_path_buf()) }
  }

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
  fn test_file_dependency_consistency() {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file();
    fs::write(&temp_file, "test1")
      .expect("failed to write to file");

    let file_dependency = FileDependency::new(temp_file.path(), FileStamper::Modified)
      .expect("failed to create file dependency");
    let dependency: Dependency<ReadStringFromFile, String> = Dependency::RequireFile(file_dependency.clone());
    assert!(file_dependency.is_inconsistent().expect("failed to check for inconsistency").is_none());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_none());

    fs::write(&temp_file, "test2")
      .expect("failed to write to file");
    assert!(file_dependency.is_inconsistent().expect("failed to check for inconsistency").is_some());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_some());
  }

  #[test]
  fn test_task_dependency_consistency() {
    let mut context = NonIncrementalContext;

    let temp_file = create_temp_file();
    fs::write(&temp_file, "test1")
      .expect("failed to write to file");
    let task = ReadStringFromFile::new(&temp_file);
    let output = context.require_task(&task);

    let task_dependency = TaskDependency::new(task.clone(), OutputStamper::Equals, output);
    let dependency = Dependency::RequireTask(task_dependency.clone());
    assert!(task_dependency.is_inconsistent(&mut context).is_none());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_none());

    fs::write(&temp_file, "test2")
      .expect("failed to write to file");
    assert!(task_dependency.is_inconsistent(&mut context).is_some());
    assert!(dependency.is_inconsistent(&mut context).expect("failed to check for inconsistency").is_some());
  }
}
