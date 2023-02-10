#![allow(clippy::wrong_self_convention, dead_code)]

use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::PathBuf;

use ron::{Deserializer, Serializer};
use ron::ser::PrettyConfig;
use serde::{Deserialize, Serialize};

use pie::{Context, Pie, Task};
use pie::stamp::FileStamper;
use pie::tracker::Tracker;
use pie::tracker::writing::WritingTracker;

fn main() {
  let mut pie = create_pie(WritingTracker::new_stdout_writer());

  pie.run_in_session(|mut session| {
    let read_task = PlaygroundTask::read_string_from_file("target/data/in.txt", FileStamper::Modified);
    let to_lower_task = PlaygroundTask::to_lower_case(read_task);
    let write_task = PlaygroundTask::write_string_to_file(to_lower_task, "target/data/out.txt", FileStamper::Modified);
    session.require(&write_task);
  });

  serialize(pie);
}


// Task implementation

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum PlaygroundTask {
  ToLowerCase(ToLowerCase),
  ReadStringFromFile(ReadStringFromFile),
  WriteStringToFile(WriteStringToFile),
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub enum PlaygroundOutput {
  ToLowerCase(Result<String, ()>),
  ReadStringFromFile(Result<String, ()>),
  WriteStringToFile(Result<(), ()>),
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ToLowerCase(pub Box<PlaygroundTask>);

impl ToLowerCase {
  fn execute<C: Context<PlaygroundTask>>(&self, context: &mut C) -> Result<String, ()> {
    let output = context.require_task(self.0.as_ref());
    let string = output.into_string()?;
    Ok(string.to_lowercase())
  }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ReadStringFromFile(pub PathBuf, pub FileStamper);

impl ReadStringFromFile {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<String, ()> {
    let mut file = context.require_file_with_stamper(&self.0, self.1).map_err(|_| ())?;
    let mut string = String::new();
    file.read_to_string(&mut string).map_err(|_| ())?;
    Ok(string)
  }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct WriteStringToFile(pub Box<PlaygroundTask>, pub PathBuf, pub FileStamper);

impl WriteStringToFile {
  fn execute<C: Context<PlaygroundTask>>(&self, context: &mut C) -> Result<(), ()> {
    let output = context.require_task(self.0.as_ref());
    let string = output.into_string()?;
    let mut file = File::create(&self.1).map_err(|_| ())?;
    file.write_all(string.as_bytes()).map_err(|_| ())?;
    context.provide_file_with_stamper(&self.1, self.2).map_err(|_| ())?;
    Ok(())
  }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Serialize, Deserialize, Debug)]
pub struct ListDirectory(pub PathBuf, pub FileStamper);

impl ListDirectory {
  fn execute<T: Task, C: Context<T>>(&self, context: &mut C) -> Result<Vec<PathBuf>, ()> {
    context.require_file_with_stamper(&self.0, self.1).map_err(|_| ())?;
    let paths = std::fs::read_dir(&self.0).map_err(|_| ())?;
    let paths: Vec<PathBuf> = paths
      .into_iter()
      .map(|p| p.unwrap().path())
      .collect();
    Ok(paths)
  }
}

impl Task for PlaygroundTask {
  type Output = PlaygroundOutput;

  fn execute<C: Context<Self>>(&self, context: &mut C) -> Self::Output {
    use PlaygroundTask::*;
    match self {
      ToLowerCase(t) => PlaygroundOutput::ToLowerCase(t.execute(context)),
      ReadStringFromFile(t) => PlaygroundOutput::ReadStringFromFile(t.execute(context)),
      WriteStringToFile(t) => PlaygroundOutput::WriteStringToFile(t.execute(context)),
    }
  }
}


// Task helpers

impl PlaygroundTask {
  pub fn read_string_from_file(path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::ReadStringFromFile(ReadStringFromFile(path.into(), stamper))
  }
  pub fn write_string_to_file(string_provider: impl Into<Box<Self>>, path: impl Into<PathBuf>, stamper: FileStamper) -> Self {
    Self::WriteStringToFile(WriteStringToFile(string_provider.into(), path.into(), stamper))
  }
  pub fn to_lower_case(string_provider: impl Into<Box<Self>>) -> Self {
    Self::ToLowerCase(ToLowerCase(string_provider.into()))
  }
}

impl PlaygroundOutput {
  pub fn into_string(self) -> Result<String, ()> {
    use PlaygroundOutput::*;
    let string = match self {
      ToLowerCase(r) => r?,
      ReadStringFromFile(r) => r?,
      o => panic!("Output {:?} does not contain a string", o),
    };
    Ok(string)
  }
}


// Pie helpers

fn create_pie<A: Tracker<PlaygroundTask> + Default>(tracker: A) -> Pie<PlaygroundTask, A> {
  let pie = Pie::<PlaygroundTask, _>::with_tracker(tracker);
  if let Ok(mut file) = File::open("target/data/pie.store") {
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Reading store file failed");
    let mut deserializer = Deserializer::from_bytes(&buffer)
      .unwrap_or_else(|e| panic!("Creating deserializer failed: {:?}", e));
    pie.deserialize(&mut deserializer).expect("Deserialization failed")
  } else {
    pie
  }
}

fn serialize<A: Tracker<PlaygroundTask> + Default>(pie: Pie<PlaygroundTask, A>) {
  let mut buffer = Vec::new();
  let mut serializer = Serializer::new(&mut buffer, Some(PrettyConfig::default()))
    .unwrap_or_else(|e| panic!("Creating serializer failed: {:?}", e));
  pie.serialize(&mut serializer).expect("Serialization failed");
  fs::create_dir_all("target/data/").expect("Creating directories for store failed");
  fs::write("target/data/pie.store", buffer).expect("Writing store failed");
}