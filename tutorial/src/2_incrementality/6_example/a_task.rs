#![allow(unused_imports, unused_variables)]

use std::io::{self, Read};
use std::path::{Path, PathBuf};

use dev_shared::{create_temp_dir, write_until_modified};
use pie::{Context, Task};
use pie::context::top_down::TopDownContext;
use pie::stamp::FileStamper;

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
