#![allow(dead_code, unused_imports)]

use dev_util::{create_temp_dir, write_until_modified};
use pie::resource::file::{FsError, ModifiedChecker};
use pie::tracker::writing::WritingTracker;
use pie::{Context, Pie, Task};
use std::fs::write;
use std::io::Read;
use std::path::PathBuf;

/// Task that reads file at `path`, returning its content as a string.
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


fn main() -> Result<(), FsError> {
  let temp_dir = create_temp_dir()?;
  let hello_file_path = temp_dir.path().join("hello.txt");
  write(&hello_file_path, "Hello, World!")?;

  let mut pie = Pie::with_tracker(WritingTracker::with_stdout());

  let task = ReadFile::new(&hello_file_path);

  // Task is executed because it is new.
  let output = pie.new_session().require(&task)?;
  println!("A: {output:?}");

  // Task is not executed, because the file has not changed.
  let output = pie.new_session().require(&task)?;
  println!("B: {output:?}");

  // Task is executed because the modified time of the file has changed (even though the file contents have not).
  write_until_modified(&hello_file_path, "Hello, World!")?;
  let output = pie.new_session().require(&task)?;
  println!("C: {output:?}");

  Ok(())
}
