use std::fs::{read_to_string, write};
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::PathBuf;
use std::rc::Rc;

use dev_util::create_temp_dir;
use pie::{Context, Pie, Task};
use pie::resource::file::{FsError, ModifiedChecker};
use pie::task::EqualsChecker;
use pie::tracker::writing::WritingTracker;

/// Task that reads file at `path` and returns its string.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct ReadFile {
  path: PathBuf,
}
impl ReadFile {
  pub fn new(path: impl Into<PathBuf>) -> Self {
    Self {
      path: path.into()
    }
  }
}
impl Task for ReadFile {
  type Output = Result<String, FsError>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut string = String::new();
    if let Some(file) = context.read(&self.path, ModifiedChecker)?.as_file() {
      file.read_to_string(&mut string)?;
    }
    Ok(string)
  }
}

/// Task that gets the string to write by first requiring `string_provider`, then writes that to file at `path`.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct WriteFile<T> {
  string_provider: T,
  path: PathBuf,
}
impl<T> WriteFile<T> {
  pub fn new(string_provider: T, path: impl Into<PathBuf>) -> Self {
    Self {
      string_provider,
      path: path.into(),
    }
  }
}
impl<T: Task<Output=Result<String, FsError>>> Task for WriteFile<T> {
  type Output = Result<(), FsError>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let string = context.require(&self.string_provider, EqualsChecker)?;
    context.write(&self.path, ModifiedChecker, |file|
      Ok(file.write_all(string.as_bytes())?),
    )?;
    Ok(())
  }
}

fn main() -> Result<(), FsError> {
  let temp_dir = create_temp_dir()?;
  let input_file_path = temp_dir.path().join("input.txt");
  write(&input_file_path, "Hello, World!")?;
  let output_file_path = temp_dir.path().join("output.txt");

  // For demonstration purposes, wrap task in an `Rc`.
  let read = Rc::new(ReadFile::new(&input_file_path));
  let write = WriteFile::new(read.clone(), &output_file_path);

  // Execute the tasks because they are new, resulting in the output file being written to.
  let mut pie = Pie::with_tracker(WritingTracker::with_stdout());
  pie.new_session().require(&write)?;
  assert_eq!(&read_to_string(&output_file_path)?, "Hello, World!");

  // Shouldn't execute anything because nothing has changed.
  pie.new_session().require(&write)?;
  pie.new_session().require(&read)?;

  Ok(())
}
