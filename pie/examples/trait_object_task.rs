use std::fs::{read_to_string, write};
use std::hash::Hash;
use std::io::{Read, Write};
use std::path::PathBuf;

use dev_util::create_temp_dir;
use pie::{Context, Pie, Task};
use pie::resource::file::{FsError, ModifiedChecker};
use pie::task::EqualsChecker;
use pie::tracker::writing::WritingTracker;
use pie::trait_object::TaskObj;

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
struct WriteFile {
  string_provider: Box<dyn TaskObj<Output=Result<String, FsError>>>,
  path: PathBuf,
}
impl WriteFile {
  pub fn new(string_provider: impl Into<Box<dyn TaskObj<Output=Result<String, FsError>>>>, path: impl Into<PathBuf>) -> Self {
    Self {
      string_provider: string_provider.into(),
      path: path.into(),
    }
  }
}
impl Task for WriteFile {
  type Output = Result<(), FsError>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    // Can require `&dyn TaskObj` with `Box::as_ref`,
    let _ = context.require_obj(self.string_provider.as_ref(), EqualsChecker)?;
    // and `&Box<dyn TaskObj>`.
    let string = context.require_obj(&self.string_provider, EqualsChecker)?;
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

  let read = ReadFile::new(&input_file_path);
  let write = WriteFile::new(read.clone(), &output_file_path);

  let mut pie = Pie::with_tracker(WritingTracker::with_stdout());

  // Execute the tasks because they are new, resulting in the output file being written to.
  pie.new_session().require(&write)?;
  assert_eq!(&read_to_string(&output_file_path)?, "Hello, World!");

  // Shouldn't execute anything because nothing has changed.
  pie.new_session().require(&write)?;
  pie.new_session().require(&read)?;

  Ok(())
}
