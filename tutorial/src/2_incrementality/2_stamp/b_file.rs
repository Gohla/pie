use std::fmt::Debug;
use std::io;
use std::path::Path;
use std::time::SystemTime;

use crate::fs::metadata;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum FileStamper {
  Exists,
  Modified,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum FileStamp {
  Exists(bool),
  Modified(Option<SystemTime>),
}

impl FileStamper {
  pub fn stamp(&self, path: impl AsRef<Path>) -> Result<FileStamp, io::Error> {
    match self {
      FileStamper::Exists => {
        Ok(FileStamp::Exists(path.as_ref().try_exists()?))
      }
      FileStamper::Modified => {
        let Some(metadata) = metadata(path)? else {
          return Ok(FileStamp::Modified(None));
        };
        Ok(FileStamp::Modified(Some(metadata.modified()?)))
      }
    }
  }
}
