use std::borrow::Borrow;
use std::collections::HashMap;
use std::error::Error;
use std::fs::File;
use std::hash::Hash;
use std::io::Read;
use std::path::PathBuf;
use std::time::SystemTime;

use anymap::AnyMap;

// Context

pub trait Context {
  fn depend<D: Dependency, B: Borrow<D>>(&mut self, dependency: B) -> D::Output;
}

// Task + implementations

pub trait Task {
  type Key: Clone + Eq + Hash + 'static;
  fn key(&self) -> &Self::Key;

  type Output: 'static;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
  fn output_equal(output_a: &Self::Output, output_b: &Self::Output) -> bool;
}

// Read file to string task

pub struct ReadFileToString {
  path: PathBuf,
}

impl Task for ReadFileToString {
  type Key = PathBuf;
  #[inline]
  fn key(&self) -> &Self::Key { &self.path }

  type Output = Result<String, std::io::Error>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = context.depend(FileDependency::new(self.path.clone()))?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
  }
  #[inline]
  fn output_equal(output_a: &Self::Output, output_b: &Self::Output) -> bool {
    match (output_a, output_b) {
      (Ok(str_a), Ok(str_b)) => str_a == str_b,
      (Err(_), Err(_)) => true,
      _ => false,
    }
  }
}

// Dependency + implementations

pub trait Dependency {
  type Output;
  fn depend<C: Context>(&self, context: &mut C) -> Self::Output;
  fn is_consistent<C: Context>(&self, context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>>;
}

// Task dependency

pub struct TaskDependency<T> {
  task: T,
}

impl<T: Task> TaskDependency<T> {
  #[inline]
  pub fn new(task: T) -> Self { Self { task } }
}

impl<T: Task> Dependency for TaskDependency<T> {
  type Output = T::Output;
  #[inline]
  fn depend<C: Context>(&self, context: &mut C) -> Self::Output {
    self.task.execute(context)
  }
  #[inline]
  fn is_consistent<C: Context>(&self, context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>> {
    if let Some(task_outputs) = store.task_outputs.get::<HashMap<T::Key, T::Output>>() {
      if let Some(previous_output) = task_outputs.get(self.task.key()) {
        let output: Self::Output = self.task.execute(context);
        return Ok(T::output_equal(&output, previous_output));
      }
    }
    Ok(false)
  }
}

// File dependency

pub struct FileDependency {
  path: PathBuf,
}

impl FileDependency {
  #[inline]
  pub fn new(path: PathBuf) -> Self { Self { path } }
  #[inline]
  fn open(&self) -> std::io::Result<File> { File::open(&self.path) }
}

impl Dependency for FileDependency {
  type Output = Result<File, std::io::Error>;
  #[inline]
  fn depend<C: Context>(&self, _context: &mut C) -> Self::Output { self.open() }
  #[inline]
  fn is_consistent<C: Context>(&self, _context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>> {
    let consistent = if let Some(previous_modified) = store.file_modification_dates.get(&self.path) {
      let modified = self.open()?.metadata()?.modified()?;
      modified == *previous_modified
    } else {
      false
    };
    Ok(consistent)
  }
}

// Store

pub struct Store {
  task_outputs: AnyMap,
  file_modification_dates: HashMap<PathBuf, SystemTime>,
}

// Naive runner

pub struct NaiveRunner {}

impl Context for NaiveRunner {
  #[inline]
  fn depend<D: Dependency, B: Borrow<D>>(&mut self, dependency: B) -> D::Output {
    dependency.borrow().depend(self)
  }
}


// Top-down incremental runner

pub struct TopDownRunner {
  // TODO: mapping from key to dependencies
}

impl Context for TopDownRunner {
  fn depend<D: Dependency, B: Borrow<D>>(&mut self, _dependency: B) -> D::Output {
    // TODO: check consistency of the dependency itself
    // TODO: check consistency of the dependencies of the dependency
    todo!()
  }
}
