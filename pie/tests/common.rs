use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Stdout, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use pie::{Context, Task};
use pie::dependency::FileStamper;
use pie::tracker::{CompositeTracker, EventTracker, WritingTracker};

// Helper functions

pub type Tracker<T> = CompositeTracker<EventTracker<T>, WritingTracker<Stdout>>;

pub fn create_tracker<T: Task>() -> Tracker<T> {
  CompositeTracker(EventTracker::new(), WritingTracker::new_stdout_writer())
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
      CommonOutput::Combine(r) => { r.check(); }
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
    let mut file = context.require_file(&self.0, self.1).map_err(|_| ())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|_| ())?;
    Ok(string)
  }
}

// Write string to file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct WriteStringToFile(pub String, pub PathBuf, pub FileStamper);

impl WriteStringToFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<(), ()> {
    let mut file = File::create(&self.1).map_err(|_| ())?;
    file.write_all(self.0.as_bytes()).map_err(|_| ())?;
    context.provide_file(&self.1, self.2).map_err(|_| ())?;
    Ok(())
  }
}

// List directory

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ListDirectory(pub PathBuf, pub FileStamper);

impl ListDirectory {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, ()> {
    context.require_file(&self.0, self.1).map_err(|_| ())?;
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
pub struct ToLowerCase(pub String);

impl ToLowerCase {
  fn execute<T: Task, C: Context<T>>(&self, _context: &mut C) -> String {
    self.0.to_lowercase()
  }
}

// Combine reading and making string to lower case

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct Combine(ReadStringFromFile);

impl Combine {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> Result<String, ()> {
    let text = match context.require_task(&CommonTask::ReadStringFromFile(self.0.clone())) { // TODO: remove clone?
      CommonOutput::ReadStringFromFile(result) => result?,
      _ => panic!(""),
    };
    match context.require_task(&CommonTask::ToLowerCase(ToLowerCase(text))) {
      CommonOutput::ToLowerCase(string) => Ok(string),
      _ => panic!(""),
    }
  }
}


// Common task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonTask {
  ReadStringFromFile(ReadStringFromFile),
  WriteStringToFile(WriteStringToFile),
  ListDirectory(ListDirectory),
  ToLowerCase(ToLowerCase),
  Combine(Combine),
  RequireSelf,
}

#[allow(dead_code)]
impl CommonTask {
  pub fn read_string_from_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ReadStringFromFile(ReadStringFromFile(path.into(), stamper))
  }
  pub fn write_string_to_file(string: impl Into<String>, path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::WriteStringToFile(WriteStringToFile(string.into(), path.into(), stamper))
  }
  pub fn list_directory(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ListDirectory(ListDirectory(path.into(), stamper))
  }
  pub fn to_lower_case(string: impl Into<String>) -> Self {
    Self::ToLowerCase(ToLowerCase(string.into()))
  }
  pub fn combine(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::Combine(Combine(ReadStringFromFile(path.into(), stamper)))
  }
  pub fn require_self() -> Self {
    Self::RequireSelf
  }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonOutput {
  ReadStringFromFile(Result<String, ()>),
  WriteStringToFile(Result<(), ()>),
  ListDirectory(Result<String, ()>),
  ToLowerCase(String),
  Combine(Result<String, ()>),
}

#[allow(dead_code)]
impl CommonOutput {
  pub fn read_string_from_file(result: Result<String, ()>) -> Self { Self::ReadStringFromFile(result) }
  pub fn read_string_from_file_ok(string: impl Into<String>) -> Self { Self::read_string_from_file(Ok(string.into())) }
  pub fn write_string_to_file(result: Result<(), ()>) -> Self { Self::WriteStringToFile(result) }
  pub fn list_directory(result: Result<String, ()>) -> Self { Self::ListDirectory(result) }
  pub fn list_directory_ok(string: impl Into<String>) -> Self { Self::list_directory(Ok(string.into())) }
  pub fn to_lower_case(string: impl Into<String>) -> Self { Self::ToLowerCase(string.into()) }
  pub fn combine(result: Result<String, ()>) -> Self { Self::Combine(result) }
  pub fn combine_ok(string: impl Into<String>) -> Self { Self::combine(Ok(string.into())) }
}

impl Task for CommonTask {
  type Output = CommonOutput;

  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    match self {
      CommonTask::ReadStringFromFile(task) => CommonOutput::ReadStringFromFile(task.execute(context)),
      CommonTask::WriteStringToFile(task) => CommonOutput::WriteStringToFile(task.execute(context)),
      CommonTask::ListDirectory(task) => CommonOutput::ListDirectory(task.execute(context)),
      CommonTask::ToLowerCase(task) => CommonOutput::ToLowerCase(task.execute(context)),
      CommonTask::Combine(task) => CommonOutput::Combine(task.execute(context)),
      CommonTask::RequireSelf => context.require_task(&CommonTask::RequireSelf),
    }
  }
}

