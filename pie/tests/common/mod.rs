use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Stdout, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use pie::{Context, Task};
use pie::stamp::FileStamper;
use pie::tracker::CompositeTracker;
use pie::tracker::event::EventTracker;
use pie::tracker::writing::WritingTracker;

// Helper functions

pub type Tracker<T> = CompositeTracker<EventTracker<T>, WritingTracker<Stdout, T>>;

pub fn create_tracker<T: Task>() -> Tracker<T> {
  CompositeTracker(EventTracker::default(), WritingTracker::new_stdout_writer())
}

pub type Pie<T> = pie::Pie<T, Tracker<T>>;

pub fn create_pie<T: Task>() -> Pie<T> {
  Pie::with_tracker(create_tracker())
}

pub fn temp_dir() -> TempDir {
  tempfile::tempdir().expect("failed to create temporary directory")
}


// Helper traits

pub trait CheckErrorExt<T> {
  fn check(self) -> T;
}

impl<T: Debug> CheckErrorExt<T> for Result<T, std::io::Error> {
  fn check(self) -> T {
    self.expect("failed io operation on file")
  }
}

impl<T: Debug> CheckErrorExt<T> for Result<T, std::io::ErrorKind> {
  fn check(self) -> T {
    self.expect("failed io operation on file")
  }
}

impl<T: Debug> CheckErrorExt<T> for Result<T, ()> {
  fn check(self) -> T {
    self.expect("something failed")
  }
}

impl CheckErrorExt<()> for CommonOutput {
  fn check(self) -> () {
    match self {
      CommonOutput::ReadStringFromFile(r) => { r.check(); }
      CommonOutput::WriteStringToFile(r) => { r.check(); }
      CommonOutput::ListDirectory(r) => { r.check(); }
      CommonOutput::ToLowerCase(r) => { r.check(); }
      _ => {}
    };
    ()
  }
}


// Pseudo-tasks

// Read string from file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ReadStringFromFile(pub PathBuf, pub FileStamper);

impl ReadStringFromFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, ()> {
    let mut file = context.require_file_with_stamper(&self.0, self.1).map_err(|_| ())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|_| ())?;
    Ok(string)
  }
}

// Write string to file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct WriteStringToFile(pub Box<CommonTask>, pub PathBuf, pub FileStamper);

impl WriteStringToFile {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<(), ()> {
    let string = match context.require_task(self.0.as_ref()) {
      CommonOutput::StringConstant(s) => s,
      CommonOutput::ReadStringFromFile(r) => r?,
      CommonOutput::ToLowerCase(r) => r?,
      _ => panic!(),
    };
    let mut file = File::create(&self.1).map_err(|_| ())?;
    file.write_all(string.as_bytes()).map_err(|_| ())?;
    context.provide_file_with_stamper(&self.1, self.2).map_err(|_| ())?;
    Ok(())
  }
}

// List directory

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ListDirectory(pub PathBuf, pub FileStamper);

impl ListDirectory {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, ()> {
    context.require_file_with_stamper(&self.0, self.1).map_err(|_| ())?;
    let paths = std::fs::read_dir(&self.0).map_err(|_| ())?;
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
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<String, ()> {
    let string = match context.require_task(self.0.as_ref()) {
      CommonOutput::StringConstant(s) => s,
      CommonOutput::ReadStringFromFile(r) => r?,
      _ => panic!(),
    };
    Ok(string.to_lowercase())
  }
}


// Common task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonTask {
  StringConstant(String),
  ReadStringFromFile(ReadStringFromFile),
  WriteStringToFile(WriteStringToFile),
  ListDirectory(ListDirectory),
  ToLowerCase(ToLowerCase),
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
  pub fn read_string_from_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ReadStringFromFile(ReadStringFromFile(path.into(), stamper))
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

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonOutput {
  StringConstant(String),
  ReadStringFromFile(Result<String, ()>),
  WriteStringToFile(Result<(), ()>),
  ListDirectory(Result<String, ()>),
  ToLowerCase(Result<String, ()>),
}

#[allow(clippy::wrong_self_convention)]
#[allow(dead_code)]
impl CommonOutput {
  pub fn string_constant(string: impl Into<String>) -> Self { Self::StringConstant(string.into()) }
  pub fn read_string_from_file(result: Result<String, ()>) -> Self { Self::ReadStringFromFile(result) }
  pub fn read_string_from_file_ok(string: impl Into<String>) -> Self { Self::read_string_from_file(Ok(string.into())) }
  pub fn write_string_to_file(result: Result<(), ()>) -> Self { Self::WriteStringToFile(result) }
  pub fn write_string_to_file_ok() -> Self { Self::WriteStringToFile(Ok(())) }
  pub fn list_directory(result: Result<String, ()>) -> Self { Self::ListDirectory(result) }
  pub fn list_directory_ok(string: impl Into<String>) -> Self { Self::list_directory(Ok(string.into())) }
  pub fn to_lower_case(result: impl Into<Result<String, ()>>) -> Self { Self::ToLowerCase(result.into()) }
  pub fn to_lower_case_ok(string: impl Into<String>) -> Self { Self::ToLowerCase(Ok(string.into())) }
}

impl Task for CommonTask {
  type Output = CommonOutput;

  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    match self {
      CommonTask::StringConstant(s) => CommonOutput::StringConstant(s.clone()),
      CommonTask::ReadStringFromFile(task) => CommonOutput::ReadStringFromFile(task.execute(context)),
      CommonTask::WriteStringToFile(task) => CommonOutput::WriteStringToFile(task.execute(context)),
      CommonTask::ListDirectory(task) => CommonOutput::ListDirectory(task.execute(context)),
      CommonTask::ToLowerCase(task) => CommonOutput::ToLowerCase(task.execute(context)),
      CommonTask::RequireSelf => context.require_task(&CommonTask::RequireSelf),
      CommonTask::RequireCycleA => context.require_task(&CommonTask::RequireCycleB),
      CommonTask::RequireCycleB => context.require_task(&CommonTask::RequireCycleA),
    }
  }
}

