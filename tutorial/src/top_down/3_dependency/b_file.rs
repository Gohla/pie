use std::fmt::Debug;
use std::fs::File;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time::SystemTime;

use crate::{Context, Task};
use crate::fs::{metadata, open_if_file};

#[derive(Clone, Eq, PartialEq, Hash, Debug)]
pub struct FileDependency {
  pub path: PathBuf,
  pub modified_date: Option<SystemTime>,
}

impl FileDependency {
  pub fn new(path: impl Into<PathBuf>) -> Result<(Self, Option<File>), io::Error> {
    let path = path.into();
    let modified_date = Self::modified_date(&path)?;
    let file = open_if_file(&path)?;
    let dependency = Self { path, modified_date };
    Ok((dependency, file))
  }

  pub fn is_inconsistent(&self) -> Result<bool, io::Error> {
    let modified_date = Self::modified_date(&self.path)?;
    Ok(modified_date != self.modified_date)
  }

  fn modified_date(path: impl AsRef<Path>) -> Result<Option<SystemTime>, io::Error> {
    let modified_date = if let Some(metadata) = metadata(path)? {
      Some(metadata.modified()?)
    } else {
      None // File does not exist -> no modified date.
    };
    Ok(modified_date)
  }
}
