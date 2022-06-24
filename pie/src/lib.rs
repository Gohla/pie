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
  fn require<D: RequirableDependency, B: Borrow<D>>(&mut self, dependency: B) -> D::Output;
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
    let mut file = context.require(FileDependency::new(self.path.clone()))?;
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

pub trait RequirableDependency {
  type Output;
  fn require<C: Context + 'static>(&self, context: &mut C, store: &mut Store) -> Self::Output;
}

pub trait CheckableDependency<C: Context> {
  fn check_consistency(&self, context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>>;
}

// Task dependency

pub struct TaskDependency<T> {
  task: T,
}

impl<T: Task> TaskDependency<T> {
  #[inline]
  pub fn new(task: T) -> Self { Self { task } }
}

impl<T: Task> RequirableDependency for TaskDependency<T> {
  type Output = T::Output;
  #[inline]
  fn require<C: Context + 'static>(&self, context: &mut C, store: &mut Store) -> Self::Output {
    context.require::<Self, _>(self)
    // if let Some(task_dependencies) = store.task_dependencies.get::<HashMap<T::Key, Vec<Box<dyn CheckableDependency<C>>>>>() {
    //   if let Some(dependencies) = task_dependencies.get(self.task.key()) {
    //     for dependency in dependencies {
    //       if !dependency.check_consistency(context, store) {}
    //     }
    //   }
    // } else {
    //   // Task has not been executed yet: execute it and
    // }
    // // TODO: check dependencies
    // // TODO: require and check output; re-execute if needed
    // self.task.execute(context)
  }
}

impl<T: Task, C: Context> CheckableDependency<C> for TaskDependency<T> {
  #[inline]
  fn check_consistency(&self, context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>> {
    if let Some(task_outputs) = store.task_outputs.get::<HashMap<T::Key, T::Output>>() {
      if let Some(previous_output) = task_outputs.get(self.task.key()) {
        let output: T::Output = context.require::<Self, _>(self);
        return Ok(T::output_equal(&output, previous_output));
      }
    }
    Ok(false) // Has not been executed before
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

impl RequirableDependency for FileDependency {
  type Output = Result<File, std::io::Error>;
  #[inline]
  fn require<C: Context>(&self, _context: &mut C, _store: &mut Store) -> Self::Output { self.open() }
}

impl<C: Context> CheckableDependency<C> for FileDependency {
  #[inline]
  fn check_consistency(&self, _context: &mut C, store: &mut Store) -> Result<bool, Box<dyn Error>> {
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
  task_dependencies: AnyMap,
  file_modification_dates: HashMap<PathBuf, SystemTime>,
}

// Naive runner

pub struct NaiveRunner {}

impl Context for NaiveRunner {
  #[inline]
  fn require<D: RequirableDependency, B: Borrow<D>>(&mut self, dependency: B) -> D::Output {
    // dependency.borrow().require(self)
    todo!()
  }
}


// Top-down incremental runner

pub struct TopDownRunner {
  // TODO: mapping from key to dependencies
}

impl Context for TopDownRunner {
  fn require<D: RequirableDependency, B: Borrow<D>>(&mut self, _dependency: B) -> D::Output {
    // TODO: check consistency of the dependency itself
    // TODO: check consistency of the dependencies of the dependency
    todo!()
  }
}
