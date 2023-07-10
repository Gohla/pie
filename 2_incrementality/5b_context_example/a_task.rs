#![allow(unused_imports, unused_variables)]

use std::fs::{File, read_to_string, remove_file, write};
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use dev_shared::{create_temp_dir, write_until_modified};
use pie::{Context, Task};
use pie::context::top_down::TopDownContext;
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
