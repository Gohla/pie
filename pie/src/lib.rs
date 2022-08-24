use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::error::Error;
use std::fs::File;
use std::hash::BuildHasher;
use std::path::PathBuf;

use task::Task;

use crate::prelude::IncrementalRunner;
use crate::store::{Store, TaskNode};
use crate::tracker::{NoopTracker, Tracker};

pub mod prelude;
pub mod task;
pub mod dependency;
pub mod runner;
pub mod store;
pub mod tracker;

pub struct Pie<C, R = NoopTracker, S = RandomState> {
  store: Store<C, S>,
  tracker: R,
}

impl Pie<IncrementalRunner<'_, '_, NoopTracker, RandomState>, NoopTracker, RandomState> {
  #[inline]
  pub fn new() -> Self { Self { store: Store::new(), tracker: NoopTracker::default() } }
}

impl<C: Context, R: Tracker, S: BuildHasher + Default> Pie<C, R, S> {
  #[inline]
  pub fn new_session(&mut self) -> Session<C, R, S> { Session::new(&mut self.store, &mut self.tracker) }
}

pub struct Session<'p, C, R, S> {
  store: &'p mut Store<C, S>,
  tracker: &'p mut R,

  visited: HashSet<TaskNode, S>,
}

impl<'p, C: Context, R: Tracker, S: BuildHasher + Default> Session<'p, C, R, S> {
  #[inline]
  fn new(store: &'p mut Store<C, S>, tracker: &'p mut R) -> Self {
    Self { store, tracker, visited: HashSet::default() }
  }
  
  pub fn require<T:Task>(&mut self, task: &T) -> Result<T::Output, (T::Output, &[Box<dyn Error>])> {
    let runner = 
  }
}


/// Incremental context, mediating between tasks and runners, enabling tasks to dynamically create dependencies that 
/// runners check for consistency and use in incremental execution.
pub trait Context {
  /// Requires given `[task]`, creating a dependency to it, and returning its up-to-date output.
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output;
  /// Requires file at given `[path]`, creating a read-dependency to the file by reading its content or metadata at the 
  /// time this function is called, and returning the opened file. Call this method *before reading from the file*.
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error>;
  /// Provides file at given `[path]`, creating a write-dependency to it by writing to its content or changing its
  /// metadata at the time this function is called. Call this method *after writing to the file*. This method does not 
  /// return the opened file, as it must be called *after writing to the file*.
  fn provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error>;
}
