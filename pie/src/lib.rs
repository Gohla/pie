use std::error::Error;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;

// Context

pub trait Context {
  fn depend<D: Dependency>(&mut self, dependency: D) -> D::Output;
}

// Task + implementations

pub trait Task {
  type Output;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
}

// Read file to string task

pub struct ReadFileToString {
  path: PathBuf,
}

impl Task for ReadFileToString {
  type Output = Result<String, Box<dyn Error>>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let mut file = context.depend(FileDependency::new(self.path.clone()))?;
    let mut string = String::new();
    file.read_to_string(&mut string)?;
    Ok(string)
  }
}

// Dependency + implementations

pub trait Dependency {
  type Output;
  fn depend<C: Context>(&self, context: &mut C) -> Self::Output;
  fn is_consistent<C: Context>(&self) -> bool;
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
  fn is_consistent<C: Context>(&self) -> bool {
    // TODO: check if input has changed
    // TODO: check if output has changed
    todo!()
  }
}

// File dependency

pub struct FileDependency {
  path: PathBuf,
}

impl FileDependency {
  #[inline]
  pub fn new(path: PathBuf) -> Self { Self { path } }
}

impl Dependency for FileDependency {
  type Output = std::io::Result<File>;
  #[inline]
  fn depend<C: Context>(&self, _context: &mut C) -> Self::Output {
    File::open(&self.path)
  }
  #[inline]
  fn is_consistent<C: Context>(&self) -> bool {
    // TODO: check if file has changed with respect to previous time
    todo!()
  }
}


// Naive runner

pub struct NaiveRunner {}

impl Context for NaiveRunner {
  #[inline]
  fn depend<D: Dependency>(&mut self, dependency: D) -> D::Output {
    dependency.depend(self)
  }
}


// Top-down incremental runner

pub struct TopDownRunner {
  // TODO: mapping from key to dependencies
}

impl Context for TopDownRunner {
  fn depend<D: Dependency>(&mut self, _dependency: D) -> D::Output {
    // TODO: check consistency of the dependency itself
    // TODO: check consistency of the dependencies of the dependency
    todo!()
  }
}
