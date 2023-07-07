use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use pie::{Context, Task};
use pie::stamp::FileStamper;

/// String constant
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct StringConstant(String);

impl StringConstant {
  #[inline]
  pub fn new(string: impl Into<String>) -> CommonTask {
    CommonTask::StringConstant(Self(string.into()))
  }
  #[inline]
  fn execute(&self) -> String {
    self.0.clone()
  }
}

/// File exists
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct FileExists(PathBuf);

impl FileExists {
  #[inline]
  pub fn new(path: impl Into<PathBuf>) -> CommonTask {
    CommonTask::FileExists(Self(path.into()))
  }
  #[inline]
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<bool, F> {
    let result = context.require_file_with_stamper(&self.0, FileStamper::Exists).map_err(|_| F)?;
    Ok(result.is_some())
  }
}

/// Read string from file
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ReadStringFromFile(PathBuf, FileStamper);

impl ReadStringFromFile {
  #[inline]
  pub fn new(path: impl Into<PathBuf>, stamper: FileStamper) -> CommonTask {
    CommonTask::ReadStringFromFile(Self(path.into(), stamper))
  }
  #[inline]
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, F> {
    let mut string = String::new();
    if let Some(mut file) = context.require_file_with_stamper(&self.0, self.1).map_err(|_| F)? {
      file.read_to_string(&mut string).map_err(|_| F)?;
    }
    Ok(string)
  }
}

/// Write string to file
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct WriteStringToFile(Box<CommonTask>, PathBuf, FileStamper);

impl WriteStringToFile {
  #[inline]
  pub fn new(string_provider: impl Into<Box<CommonTask>>, path: impl Into<PathBuf>, stamper: FileStamper) -> CommonTask {
    CommonTask::WriteStringToFile(Self(string_provider.into(), path.into(), stamper))
  }
  #[inline]
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), F> {
    let string = context.require_task(&self.0)?.into_string();
    let mut file = File::create(&self.1).map_err(|_| F)?;
    file.write_all(string.as_bytes()).map_err(|_| F)?;
    context.provide_file_with_stamper(&self.1, self.2).map_err(|_| F)?;
    Ok(())
  }
}

/// List directory
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ListDirectory(PathBuf, FileStamper);

impl ListDirectory {
  #[inline]
  pub fn new(path: impl Into<PathBuf>, stamper: FileStamper) -> CommonTask {
    CommonTask::ListDirectory(Self(path.into(), stamper))
  }
  #[inline]
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, F> {
    context.require_file_with_stamper(&self.0, self.1).map_err(|_| F)?;
    let paths = std::fs::read_dir(&self.0).map_err(|_| F)?;
    let paths: String = paths
      .into_iter()
      .map(|p| p.unwrap().path().to_string_lossy().to_string())
      .fold(String::new(), |a, b| a + &b + "\n");
    Ok(paths)
  }
}

/// Make string lowercase
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ToLowerCase(Box<CommonTask>);

impl ToLowerCase {
  #[inline]
  pub fn new(string_provider: impl Into<Box<CommonTask>>) -> CommonTask {
    CommonTask::ToLowerCase(Self(string_provider.into()))
  }
  #[inline]
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<String, F> {
    let string = context.require_task(self.0.as_ref())?.into_string();
    Ok(string.to_lowercase())
  }
}

/// Make string uppercase
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ToUpperCase(Box<CommonTask>);

impl ToUpperCase {
  #[inline]
  pub fn new(string_provider: impl Into<Box<CommonTask>>) -> CommonTask {
    CommonTask::ToUpperCase(Self(string_provider.into()))
  }
  #[inline]
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<String, F> {
    let string = context.require_task(self.0.as_ref())?.into_string();
    Ok(string.to_uppercase())
  }
}

/// Require a task when a file exists
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct RequireTaskOnFileExists(Box<CommonTask>, PathBuf);

impl RequireTaskOnFileExists {
  #[inline]
  pub fn new(task: impl Into<Box<CommonTask>>, path: impl Into<PathBuf>) -> CommonTask {
    CommonTask::RequireTaskOnFileExists(Self(task.into(), path.into()))
  }
  #[inline]
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), F> {
    if let Some(_) = context.require_file_with_stamper(&self.1, FileStamper::Exists).map_err(|_| F)? {
      context.require_task(&self.0)?;
    }
    Ok(())
  }
}

/// Sequence
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct Sequence(Vec<CommonTask>);

impl Sequence {
  #[inline]
  pub fn new(tasks: impl Into<Vec<CommonTask>>) -> CommonTask {
    CommonTask::Sequence(Self(tasks.into()))
  }
  #[inline]
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), F> {
    for task in &self.0 {
      context.require_task(task)?;
    }
    Ok(())
  }
}


/// Common task enumeration
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonTask {
  StringConstant(StringConstant),
  FileExists(FileExists),
  ReadStringFromFile(ReadStringFromFile),
  WriteStringToFile(WriteStringToFile),
  ListDirectory(ListDirectory),
  ToLowerCase(ToLowerCase),
  ToUpperCase(ToUpperCase),
  RequireTaskOnFileExists(RequireTaskOnFileExists),
  Sequence(Sequence),
}

impl Task for CommonTask {
  type Output = Result<CommonOutput, F>;
  #[inline]
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    use CommonTask::*;
    match self {
      StringConstant(t) => Ok(t.execute().into()),
      FileExists(t) => t.execute(context).map(Into::into),
      ReadStringFromFile(t) => t.execute(context).map(Into::into),
      WriteStringToFile(t) => t.execute(context).map(Into::into),
      ListDirectory(t) => t.execute(context).map(Into::into),
      ToLowerCase(t) => t.execute(context).map(Into::into),
      ToUpperCase(t) => t.execute(context).map(Into::into),
      RequireTaskOnFileExists(t) => t.execute(context).map(Into::into),
      Sequence(t) => t.execute(context).map(Into::into),
    }
  }
}

impl From<&CommonTask> for Box<CommonTask> {
  fn from(t: &CommonTask) -> Self { Box::new(t.clone()) }
}


/// Common output enumeration
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonOutput {
  String(String),
  Bool(bool),
  Unit,
}

impl CommonOutput {
  #[inline]
  pub fn as_str(&self) -> &str {
    match self {
      Self::String(s) => &s,
      o => panic!("Output {:?} does not contain a string", o),
    }
  }
  #[inline]
  pub fn into_string(self) -> String {
    match self {
      Self::String(s) => s,
      o => panic!("Output {:?} does not contain a string", o),
    }
  }
}

impl From<String> for CommonOutput {
  #[inline]
  fn from(value: String) -> Self { Self::String(value) }
}

impl From<bool> for CommonOutput {
  #[inline]
  fn from(value: bool) -> Self { Self::Bool(value) }
}

impl From<()> for CommonOutput {
  #[inline]
  fn from(_: ()) -> Self { Self::Unit }
}


/// Serializable failure type. We can't use [`std::io::ErrorKind`] because that is not serializable and we cannot
/// implement serialization for it due to the orphan rule. We can't use `()` because that cannot be converted to 
/// `Box<dyn Error>`.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize)]
pub struct F;

impl Debug for F {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { f.write_str("something failed") }
}

impl Display for F {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { Debug::fmt(self, f) }
}

impl Error for F {}
