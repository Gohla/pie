use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use pie::{Context, Task};
use pie::stamp::FileStamper;

// File exists

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct FileExists(pub PathBuf);

impl FileExists {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<bool, F> {
    let result = context.require_file_with_stamper(&self.0, FileStamper::Exists).map_err(|_| F)?;
    Ok(result.is_some())
  }
}

// Read string from file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ReadStringFromFile(pub PathBuf, pub FileStamper);

impl ReadStringFromFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, F> {
    let mut string = String::new();
    if let Some(mut file) = context.require_file_with_stamper(&self.0, self.1).map_err(|_| F)? {
      file.read_to_string(&mut string).map_err(|_| F)?;
    }
    Ok(string)
  }
}

// Read indirect string from file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ReadIndirectStringFromFile(pub PathBuf, pub FileStamper);

impl ReadIndirectStringFromFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, F> {
    let mut string = String::new();
    if let Some(mut file) = context.require_file_with_stamper(&self.0, self.1).map_err(|_| F)? {
      let mut indirect_path = String::new();
      file.read_to_string(&mut indirect_path).map_err(|_| F)?;
      let indirect_path = PathBuf::from(indirect_path);
      if let Some(mut file) = context.require_file_with_stamper(&indirect_path, self.1).map_err(|_| F)? {
        file.read_to_string(&mut string).map_err(|_| F)?;
      }
    }
    Ok(string)
  }
}

// Write string to file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct WriteStringToFile(pub Box<CommonTask>, pub PathBuf, pub FileStamper);

impl WriteStringToFile {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), F> {
    let string = context.require_task(&self.0)?.into_string();
    let mut file = File::create(&self.1).map_err(|_| F)?;
    file.write_all(string.as_bytes()).map_err(|_| F)?;
    context.provide_file_with_stamper(&self.1, self.2).map_err(|_| F)?;
    Ok(())
  }
}

// List directory

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ListDirectory(pub PathBuf, pub FileStamper);

impl ListDirectory {
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

// Make string lowercase

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ToLowerCase(pub Box<CommonTask>);

impl ToLowerCase {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<String, F> {
    let string = context.require_task(self.0.as_ref())?.into_string();
    Ok(string.to_lowercase())
  }
}

// Make string uppercase

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ToUpperCase(pub Box<CommonTask>);

impl ToUpperCase {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<String, F> {
    let string = context.require_task(self.0.as_ref())?.into_string();
    Ok(string.to_uppercase())
  }
}

// Require a task when a file exists

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct RequireTaskOnFileExists(pub Box<CommonTask>, pub PathBuf);

impl RequireTaskOnFileExists {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), F> {
    if let Some(_) = context.require_file_with_stamper(&self.1, FileStamper::Exists).map_err(|_| F)? {
      context.require_task(&self.0)?;
    }
    Ok(())
  }
}

// Sequence

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct Sequence(pub Vec<Box<CommonTask>>);

impl Sequence {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), F> {
    for task in &self.0 {
      context.require_task(task)?;
    }
    Ok(())
  }
}


// Common task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonTask {
  StringConstant(String),
  FileExists(FileExists),
  ReadStringFromFile(ReadStringFromFile),
  ReadIndirectStringFromFile(ReadIndirectStringFromFile),
  WriteStringToFile(WriteStringToFile),
  ListDirectory(ListDirectory),
  ToLowerCase(ToLowerCase),
  ToUpperCase(ToUpperCase),
  RequireTaskOnFileExists(RequireTaskOnFileExists),
  Sequence(Sequence),
  RequireSelf,
  RequireCycleA,
  RequireCycleB,
}

#[allow(clippy::wrong_self_convention)]
#[allow(dead_code)]
impl CommonTask {
  pub fn string_constant(string: impl Into<String>) -> Self {
    Self::StringConstant(string.into())
  }
  pub fn file_exists(path: impl Into<PathBuf>) -> Self {
    Self::FileExists(FileExists(path.into()))
  }
  pub fn read_string_from_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ReadStringFromFile(ReadStringFromFile(path.into(), stamper))
  }
  pub fn read_indirect_string_from_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ReadIndirectStringFromFile(ReadIndirectStringFromFile(path.into(), stamper))
  }
  pub fn write_string_to_file(string_provider: impl Into<Box<CommonTask>>, path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::WriteStringToFile(WriteStringToFile(string_provider.into(), path.into(), stamper))
  }
  pub fn write_constant_string_to_file(string: impl Into<String>, path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::WriteStringToFile(WriteStringToFile(Box::new(CommonTask::string_constant(string)), path.into(), stamper))
  }
  pub fn list_directory(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ListDirectory(ListDirectory(path.into(), stamper))
  }
  pub fn to_lower_case(string_provider: impl Into<Box<CommonTask>>) -> Self {
    Self::ToLowerCase(ToLowerCase(string_provider.into()))
  }
  pub fn to_lower_case_constant(string: impl Into<String>) -> Self {
    Self::ToLowerCase(ToLowerCase(Box::new(Self::string_constant(string))))
  }
  pub fn to_upper_case(string_provider: impl Into<Box<CommonTask>>) -> Self {
    Self::ToUpperCase(ToUpperCase(string_provider.into()))
  }
  pub fn require_task_on_file_exists(task: impl Into<Box<CommonTask>>, path: impl Into<PathBuf>) -> Self {
    Self::RequireTaskOnFileExists(RequireTaskOnFileExists(task.into(), path.into()))
  }
  pub fn sequence(tasks: impl Into<Vec<CommonTask>>) -> Self {
    let tasks: Vec<Box<CommonTask>> = tasks.into().into_iter().map(|t| Box::new(t)).collect();
    Self::Sequence(Sequence(tasks))
  }

  pub fn require_self() -> Self {
    Self::RequireSelf
  }
  pub fn require_cycle_a() -> Self {
    Self::RequireCycleA
  }
  pub fn require_cycle_b() -> Self {
    Self::RequireCycleB
  }
}

impl Task for CommonTask {
  type Output = Result<CommonOutput, F>;

  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    use CommonTask::*;
    use CommonOutput::*;
    match self {
      StringConstant(s) => Ok(String(s.clone())),
      FileExists(task) => task.execute(context).map(|b| Bool(b)),
      ReadStringFromFile(task) => task.execute(context).map(|s| String(s)),
      ReadIndirectStringFromFile(task) => task.execute(context).map(|s| String(s)),
      WriteStringToFile(task) => task.execute(context).map(|_| Unit),
      ListDirectory(task) => task.execute(context).map(|s| String(s)),
      ToLowerCase(task) => task.execute(context).map(|s| String(s)),
      ToUpperCase(task) => task.execute(context).map(|s| String(s)),
      RequireTaskOnFileExists(task) => task.execute(context).map(|_| Unit),
      Sequence(task) => task.execute(context).map(|_| Unit),
      RequireSelf => context.require_task(&RequireSelf),
      RequireCycleA => context.require_task(&RequireCycleB),
      RequireCycleB => context.require_task(&RequireCycleA),
    }
  }
}


// Common output

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonOutput {
  String(String),
  Bool(bool),
  Unit,
}

#[allow(dead_code)]
impl CommonOutput {
  #[inline]
  pub fn new_string(string: impl Into<String>) -> Self { Self::String(string.into()) }
  #[inline]
  pub const fn new_bool(bool: bool) -> Self { Self::Bool(bool) }
  #[inline]
  pub const fn new_unit() -> Self { Self::Unit }

  #[inline]
  pub fn as_str(&self) -> &str {
    match self {
      CommonOutput::String(s) => &s,
      o => panic!("Output {:?} does not contain a string", o),
    }
  }

  #[inline]
  pub fn into_string(self) -> String {
    match self {
      CommonOutput::String(s) => s,
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

impl AsRef<str> for CommonOutput {
  #[inline]
  fn as_ref(&self) -> &str { self.as_str() }
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
