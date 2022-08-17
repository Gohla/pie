use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use pie::Context;
use pie::task::Task;

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
    println!("ReadStringFromFile {:?}", self);
    let mut file = context.require_file(&self.path).map_err(|e| e.kind())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|e| e.kind())?;
    Ok(string)
  }
}


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
    println!("WriteBytesToFile {:?}", self);
    let mut file = File::create(&self.path).map_err(|e| e.kind())?;
    file.write_all(self.string.as_bytes()).map_err(|e| e.kind())?;
    context.provide_file(&self.path).map_err(|e| e.kind())?;
    Ok(())
  }
}
