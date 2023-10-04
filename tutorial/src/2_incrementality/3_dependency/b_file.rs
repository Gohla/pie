use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::{Context, Task};
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
