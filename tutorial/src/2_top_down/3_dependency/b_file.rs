use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::path::PathBuf;

use crate::{Context, Task};
use crate::fs::open_if_file;
use crate::stamp::{FileStamp, FileStamper, OutputStamp, OutputStamper};

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct FileDependency {
  pub path: PathBuf,
  pub stamper: FileStamper,
  pub stamp: FileStamp,
}

impl FileDependency {
  pub fn new(path: impl Into<PathBuf>, stamper: FileStamper) -> Result<(Self, Option<File>), io::Error> {
    let path = path.into();
    let stamp = stamper.stamp(&path)?;
    let file = open_if_file(&path)?;
    let dependency = FileDependency { path, stamper, stamp };
    Ok((dependency, file))
  }

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
