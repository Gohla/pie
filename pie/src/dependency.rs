use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::{Context, Task};

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub(crate) enum Dependency<T, O> {
  RequireFile(PathBuf, FileStamper, FileStamp),
  ProvideFile(PathBuf, FileStamper, FileStamp),
  RequireTask(T, O),
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

  pub fn require_task(task: T, output: T::Output) -> Self {
    Self::RequireTask(task, output)
  }

  pub fn is_consistent<C: Context<T>>(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    match self {
      Dependency::RequireFile(path, stamper, stamp) => Self::file_is_consistent(path, stamper, stamp),
      Dependency::ProvideFile(path, stamper, stamp) => Self::file_is_consistent(path, stamper, stamp),
      Dependency::RequireTask(task, output) => Ok(context.require_task(task) == *output),
    }
  }

  fn file_is_consistent(path: &PathBuf, stamper: &FileStamper, stamp: &FileStamp) -> Result<bool, Box<dyn Error>> {
    let new_stamp = stamper.stamp(path)?;
    Ok(new_stamp == *stamp)
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum FileStamper {
  Exists,
  Modified,
  ModifiedRecursive,
  Hash,
  HashRecursive,
}

impl FileStamper {
  pub fn stamp(&self, path: &PathBuf) -> Result<FileStamp, std::io::Error> {
    match self {
      FileStamper::Exists => {
        Ok(FileStamp::Exists(path.try_exists()?))
      }
      FileStamper::Modified => {
        Ok(FileStamp::Modified(File::open(path)?.metadata()?.modified()?))
      }
      FileStamper::ModifiedRecursive => {
        let mut latest_modification_date = SystemTime::UNIX_EPOCH;
        for entry in WalkDir::new(path).into_iter() {
          let entry_modification_date = entry?.metadata()?.modified()?;
          if entry_modification_date > latest_modification_date {
            latest_modification_date = entry_modification_date;
          }
        }
        Ok(FileStamp::Modified(latest_modification_date))
      }
      FileStamper::Hash => {
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        std::io::copy(&mut file, &mut hasher)?;
        Ok(FileStamp::Hash(hasher.finalize().into()))
      }
      FileStamper::HashRecursive => {
        let mut hasher = Sha256::new();
        for entry in WalkDir::new(path).into_iter() {
          let mut file = File::open(entry?.path())?;
          if !file.metadata()?.is_file() { continue; }
          std::io::copy(&mut file, &mut hasher)?;
        }
        Ok(FileStamp::Hash(hasher.finalize().into()))
      }
    }
  }
}

#[derive(Clone, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum FileStamp {
  Exists(bool),
  Modified(SystemTime),
  Hash([u8; 32]),
}
