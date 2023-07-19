#![allow(unused_imports, unused_variables)]

use std::fs::{File, read_to_string, remove_file, write};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use dev_shared::{create_temp_dir, write_until_modified};
use pie::{Context, Pie, Task};
use pie::stamp::FileStamper;

/// Enumeration over file pseudo-tasks.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
enum FileTask {
  ReadStringFromFile(ReadStringFromFile),
  WriteStringToFile(WriteStringToFile),
}

/// [`Task`] implementation for [`FileTask`], forwarding execute to the execute functions of the pseudo-tasks.
impl Task for FileTask {
  type Output = Result<String, io::ErrorKind>;
  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    match self {
      FileTask::ReadStringFromFile(t) => t.execute(context),
      FileTask::WriteStringToFile(t) => t.execute(context).map(|_| String::new())
    }
  }
}

/// Pseudo-task that reads a string from a file.
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct ReadStringFromFile(PathBuf, FileStamper);

impl ReadStringFromFile {
  fn new(path: impl AsRef<Path>, stamper: FileStamper) -> FileTask {
    FileTask::ReadStringFromFile(Self(path.as_ref().to_path_buf(), stamper))
  }
  fn execute<C: Context<FileTask>>(&self, context: &mut C) -> Result<String, io::ErrorKind> {
    println!("Reading from {} with {:?} stamper", self.0.file_name().unwrap().to_string_lossy(), self.1);
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

/// Pseudo-task that writes a string to a file, where the string is provided by another task. The string provider is 
/// boxed to prevent a cyclic definition of infinite size, due to this type being used in [`FileTask`].
#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct WriteStringToFile(Box<FileTask>, PathBuf, FileStamper);

impl WriteStringToFile {
  fn new(string_provider: impl Into<Box<FileTask>>, path: impl Into<PathBuf>, stamper: FileStamper) -> FileTask {
    FileTask::WriteStringToFile(Self(string_provider.into(), path.into(), stamper))
  }
  fn execute<C: Context<FileTask>>(&self, context: &mut C) -> Result<(), io::ErrorKind> {
    println!("Writing to {} with {:?} stamper", self.1.file_name().unwrap().to_string_lossy(), self.2);
    let string = context.require_task(&self.0)?;
    let mut file = File::create(&self.1).map_err(|e| e.kind())?;
    file.write_all(string.as_bytes()).map_err(|e| e.kind())?;
    context.require_file_with_stamper(&self.1, self.2).map_err(|e| e.kind())?;
    Ok(())
  }
}

fn main() -> Result<(), io::Error> {
  let temp_dir = create_temp_dir()?;
  let input_file = temp_dir.path().join("input.txt");
  write(&input_file, "Hi")?;
  let output_file = temp_dir.path().join("output.txt");

  let mut pie = Pie::default();
  let read_task = ReadStringFromFile::new(&input_file, FileStamper::Modified);
  let write_task = WriteStringToFile::new(read_task.clone(), &output_file, FileStamper::Modified);

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

  println!("\nD) New task, reuse other: expect only `write_task` to execute");
  // write_task` is new, but `read_task` is not new and its file dependency is still consistent.
  pie.new_session().require(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello");

  write_until_modified(&input_file, "Hello, World!")?;
  println!("\nE) Inconsistent file and task dependency: expect both tasks to execute");
  // The file dependency of `read_task` is inconsistent. Then, the task dependency from `write_task` to `read_task` is 
  // inconsistent because `read_task` now returns `"Hello, World!"` as output instead of "Hello", and thus its equals 
  // output stamp is different.
  pie.new_session().require(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

  write_until_modified(&input_file, "Hello, World!")?; // Note: writing same file contents!
  println!("\nF) Early cutoff: expect only `read_task` to execute");
  // File dependency of `read_task` is inconsistent because the modified time changed, but it returns the same output 
  // `"Hello, World!"` because the contents of the file have not actually changed. Then, the task dependency from 
  // `write_task` to `read_task` is consistent because its output did not change, and thus the equality output stamp is 
  // the same.
  pie.new_session().require(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

  write_until_modified(&output_file, "")?;
  println!("\nG) Regenerate changed output file: expect only `write_task` to execute");
  // The file dependency of `write_task` to `output_file` is inconsistent.
  pie.new_session().require(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

  write_until_modified(&output_file, "")?;
  remove_file(&output_file)?;
  println!("\nH) Regenerate deleted output file: expect only `write_task` to execute");
  // Same results when `output_file` is deleted.
  pie.new_session().require(&write_task)?;
  assert_eq!(&read_to_string(&output_file)?, "Hello, World!");

  let input_file_b = temp_dir.path().join("input_b.txt");
  write(&input_file_b, "Test")?;
  let read_task_b_modified = ReadStringFromFile::new(&input_file_b, FileStamper::Modified);
  let read_task_b_exists = ReadStringFromFile::new(&input_file_b, FileStamper::Exists);
  println!("\nI) Different tasks: expect `read_task_b_modified` and `read_task_b_exists` to execute");
  // Task `read_task`, `read_task_b_modified` and `read_task_b_exists` are different, due to their `Eq` implementation 
  // determining that their paths and stampers are different. Therefore, `read_task_b_modified` and `read_task_b_exists`
  // are new tasks, and must be executed.
  let mut session = pie.new_session();
  let output = session.require(&read_task_b_modified)?;
  assert_eq!(&output, "Test");
  let output = session.require(&read_task_b_exists)?;
  assert_eq!(&output, "Test");

  write_until_modified(&input_file_b, "Test Test")?;
  println!("\nJ) Different stampers: expect only `read_task_b_modified` to execute");
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
