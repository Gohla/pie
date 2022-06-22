use std::fs::File;
use std::path::PathBuf;

// Context

pub trait Context {
  fn depend<D: Dependency>(&mut self, dependency: D) -> D::Output;
}


// Task

pub trait Task {
  type Output;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
}


// Dependency + implementations

pub trait Dependency {
  type Output;
  fn depend<C: Context>(self, context: &mut C) -> Self::Output;
}

// Task dependency

pub struct TaskDependency<T> {
  task: T,
}

impl<T: Task> Dependency for TaskDependency<T> {
  type Output = T::Output;
  fn depend<C: Context>(self, context: &mut C) -> Self::Output {
    self.task.execute(context)
  }
}

// File dependency

pub struct FileDependency {
  path: PathBuf,
}

impl Dependency for FileDependency {
  type Output = std::io::Result<File>;

  fn depend<C: Context>(self, _context: &mut C) -> Self::Output {
    File::open(&self.path)
  }
}


// Naive runner

pub struct NaiveRunner {}

impl Context for NaiveRunner {
  fn depend<D: Dependency>(&mut self, dependency: D) -> D::Output {
    dependency.depend(self)
  }
}
