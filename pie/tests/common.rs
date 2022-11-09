use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Stdout, Write};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tempfile::TempDir;

use pie::prelude::*;
use pie::tracker::{CompositeTracker, EventTracker, WritingTracker};

// Helper functions

pub type Tracker<T> = CompositeTracker<EventTracker<T>, WritingTracker<Stdout>>;

pub fn create_tracker<T: Task>() -> Tracker<T> {
  CompositeTracker(EventTracker::new(), WritingTracker::new_stdout_writer())
}

pub type Pie<T: Task> = pie::Pie<T, T::Output, Tracker<T>>;

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


// Pseudo-tasks

// Read string from file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ReadStringFromFile(pub PathBuf);

impl ReadStringFromFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, ()> {
    let mut file = context.require_file(&self.0).map_err(|_| ())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|_| ())?;
    Ok(string)
  }
}

// Write string to file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct WriteStringToFile(pub PathBuf, pub String);

impl WriteStringToFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<(), ()> {
    let mut file = File::create(&self.0).map_err(|_| ())?;
    file.write_all(self.1.as_bytes()).map_err(|_| ())?;
    context.provide_file(&self.0).map_err(|_| ())?;
    Ok(())
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

#[derive(Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Debug)]
struct Combine(ReadStringFromFile);

impl Combine {
  fn execute<C: Context<CommonTask>>(&self, context: &mut C) -> CommonOutput {
    let text = context.require_task(&CommonTask::ReadStringFromFile(self.0.clone()))?; // TODO: remove clone?
    CommonOutput::Combine(Ok(context.require_task(&CommonTask::ToLowerCase(ToLowerCase(text)))))
  }
}


// Common task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonTask {
  ReadStringFromFile(ReadStringFromFile),
  WriteStringToFile(WriteStringToFile),
  ToLowerCase(ToLowerCase),
  Combine(ReadStringFromFile)
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum CommonOutput {
  ReadStringFromFile(Result<String, ()>),
  WriteStringToFile(Result<(), ()>),
  ToLowerCase(String),
  Combine(Result<String, ()>),
}

impl CommonOutput {
  pub fn to_lower_case(string: impl Into<String>) -> Self {
    Self::ToLowerCase(string.into())
  }
}

impl Task for CommonTask {
  type Output = CommonOutput;

  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    match self {
      CommonTask::ReadStringFromFile(task) => CommonOutput::ReadStringFromFile(task.execute(context)),
      CommonTask::WriteStringToFile(task) => CommonOutput::WriteStringToFile(task.execute(context)),
      CommonTask::ToLowerCase(task) => CommonOutput::ToLowerCase(task.execute(context)),
      CommonTask::Combine(read_string_from_file) => {
        let text = context.require_task(&CommonTask::ReadStringFromFile(read_string_from_file.clone()))?; // TODO: remove clone?
        Ok(context.require_task(&CommonTask::ToLowerCase(ToLowerCase(text))))
      }
    }
  }
}

