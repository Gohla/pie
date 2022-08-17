use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Stdout, Write};
use std::path::PathBuf;

use tempfile::TempDir;

use pie::Context;
use pie::runner::topdown::TopDownRunner;
use pie::task::Task;
use pie::tracker::WritingTracker;

// Helper functions

pub fn create_runner() -> TopDownRunner<WritingTracker<Stdout>> {
  TopDownRunner::with_tracker(WritingTracker::new_stdout_writer())
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
pub struct ReadStringFromFile {
  path: PathBuf,
}

impl ReadStringFromFile {
  pub fn new(path: PathBuf) -> Self { Self { path } }
}

impl Task for ReadStringFromFile {
  type Output = Result<String, std::io::ErrorKind>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = context.require_file(&self.path).map_err(|e| e.kind())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|e| e.kind())?;
    Ok(string)
  }
}


// Write string to file task

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct WriteStringToFile {
  path: PathBuf,
  string: String,
}

impl WriteStringToFile {
  pub fn new(path: PathBuf, string: &str) -> Self { Self { path, string: string.to_string() } }
}

impl Task for WriteStringToFile {
  type Output = Result<(), std::io::ErrorKind>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = File::create(&self.path).map_err(|e| e.kind())?;
    file.write_all(self.string.as_bytes()).map_err(|e| e.kind())?;
    context.provide_file(&self.path).map_err(|e| e.kind())?;
    Ok(())
  }
}
