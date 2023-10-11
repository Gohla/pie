use std::io::{self, Read};
use std::path::{Path, PathBuf};

use dev_shared::{create_temp_dir, write_until_modified};
use pie::{Context, Pie, Task};
use pie::stamp::FileStamper;
use pie::tracker::writing::WritingTracker;

/// Task that reads a string from a file.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct ReadStringFromFile(PathBuf, FileStamper);

impl ReadStringFromFile {
  fn new(path: impl AsRef<Path>, stamper: FileStamper) -> Self {
    Self(path.as_ref().to_path_buf(), stamper)
  }
}

impl Task for ReadStringFromFile {
  type Output = Result<String, io::ErrorKind>;
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    let file = context.require_file_with_stamper(&self.0, self.1).map_err(|e| e.kind())?;
    if let Some(mut file) = file {
      let mut string = String::new();
      file.read_to_string(&mut string).map_err(|e| e.kind())?;
      Ok(string)
    } else {
      Err(io::ErrorKind::NotFound)
    }
  }
}

fn main() -> Result<(), io::Error> {
  let temp_dir = create_temp_dir()?;
  let input_file = temp_dir.path().join("input.txt");
  write_until_modified(&input_file, "Hi")?;

  let mut pie = Pie::with_tracker(WritingTracker::with_stdout());
  let read_task = ReadStringFromFile::new(&input_file, FileStamper::Modified);

  println!("A) New task: expect `read_task` to execute");
  // `read_task` is new, meaning that we have no cached output for it, thus it must be executed.
  let output = pie.new_session().require(&read_task)?;
  assert_eq!(&output, "Hi");

  println!("\nB) Reuse: expect no execution");
  // `read_task` is not new and its file dependency is still consistent. It is consistent because the modified time of
  // `input_file` has not changed, thus the modified stamp is equal.
  let output = pie.new_session().require(&read_task)?;
  assert_eq!(&output, "Hi");

  write_until_modified(&input_file, "Hello")?;
  println!("\nC) Inconsistent file dependency: expect `read_task` to execute");
  // The file dependency of `read_task` is inconsistent due to the changed modified time of `input_file`.
  let output = pie.new_session().require(&read_task)?;
  assert_eq!(&output, "Hello");

  let input_file_b = temp_dir.path().join("input_b.txt");
  write_until_modified(&input_file_b, "Test")?;
  let read_task_b_modified = ReadStringFromFile::new(&input_file_b, FileStamper::Modified);
  let read_task_b_exists = ReadStringFromFile::new(&input_file_b, FileStamper::Exists);
  println!("\nD) Different tasks: expect `read_task_b_modified` and `read_task_b_exists` to execute");
  // Task `read_task`, `read_task_b_modified` and `read_task_b_exists` are different, due to their `Eq` implementation
  // determining that their paths and stampers are different. Therefore, `read_task_b_modified` and `read_task_b_exists`
  // are new tasks, and must be executed.
  let mut session = pie.new_session();
  let output = session.require(&read_task_b_modified)?;
  assert_eq!(&output, "Test");
  let output = session.require(&read_task_b_exists)?;
  assert_eq!(&output, "Test");

  write_until_modified(&input_file_b, "Test Test")?;
  println!("\nE) Different stampers: expect only `read_task_b_modified` to execute");
  // Both `read_task_b_modified` and `read_task_b_exists` read from the same file, but they use different stampers.
  // Therefore, `read_task_b_modified` must be executed because the modified time has changed, but `read_task_b_exists`
  // will not be executed because its file dependency stamper only checks for existence of the file, and the existence
  // of the file has not changed.
  //
  // Note that using an `Exists` stamper for this task does not make a lot of sense, since it will only read the file
  // on first execute and when it is recreated. But this is just to demonstrate different stampers.
  let mut session = pie.new_session();
  let output = session.require(&read_task_b_modified)?;
  assert_eq!(&output, "Test Test");
  let output = session.require(&read_task_b_exists)?;
  assert_eq!(&output, "Test");

  Ok(())
}
