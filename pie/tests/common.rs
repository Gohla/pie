use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Stdout, Write};
use std::path::PathBuf;

use tempfile::TempDir;

use pie::prelude::*;
use pie::tracker::{CompositeTracker, EventTracker, WritingTracker};

// Helper functions

pub fn create_tracker() -> CompositeTracker<EventTracker, WritingTracker<Stdout>> {
  CompositeTracker(EventTracker::new(), WritingTracker::new_stdout_writer())
}

pub fn create_runner() -> TopDownRunner<CompositeTracker<EventTracker, WritingTracker<Stdout>>> {
  TopDownRunner::with_tracker(create_tracker())
}

pub fn temp_dir() -> TempDir {
  tempfile::tempdir().expect("failed to create temporary directory")
}


// Helper traits

pub trait CheckErrorExt<T> {
  fn check(self) -> T;
}

impl<T: Debug> CheckErrorExt<T> for Result<T, (T, &[Box<dyn Error>])> {
  fn check(self) -> T {
    self.expect("failed to check one or more dependencies")
  }
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


// Read string from file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ReadStringFromFile(pub PathBuf);

impl Task for ReadStringFromFile {
  type Output = Result<String, std::io::ErrorKind>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = context.require_file(&self.0).map_err(|e| e.kind())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|e| e.kind())?;
    Ok(string)
  }
}


// Write string to file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct WriteStringToFile(pub PathBuf, pub String);

impl Task for WriteStringToFile {
  type Output = Result<(), std::io::ErrorKind>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = File::create(&self.0).map_err(|e| e.kind())?;
    file.write_all(self.1.as_bytes()).map_err(|e| e.kind())?;
    context.provide_file(&self.0).map_err(|e| e.kind())?;
    Ok(())
  }
}


// Make string lowercase

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ToLowerCase(pub String);

impl Task for ToLowerCase {
  type Output = String;
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
    self.0.to_lowercase()
  }
}
