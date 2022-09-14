use std::any::Any;
use std::error::Error;
use std::fmt::Debug;
use std::fs::File;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use dyn_clone::DynClone;

use crate::{Context, Output, Task};
use crate::dependency::Dependency;

/// Object-safe version of [`Task`].
pub trait DynTask: DynClone + Any + Debug {
  fn dyn_execute(&self, context: &mut dyn DynContext) -> Box<dyn DynOutput>;
  fn dyn_eq(&self, other: &dyn Any) -> bool;
  fn dyn_hash(&self, state: &mut dyn Hasher);
  fn as_any(&self) -> &dyn Any;
}

impl<T: Task> DynTask for T {
  #[inline]
  fn dyn_execute(&self, mut context: &mut dyn DynContext) -> Box<dyn DynOutput> {
    Box::new(self.execute(&mut context))
  }
  #[inline]
  fn dyn_eq(&self, other: &dyn Any) -> bool {
    other.downcast_ref::<Self>().map_or(false, |o| self == o)
  }
  #[inline]
  fn dyn_hash(&self, mut state: &mut dyn Hasher) { self.hash(&mut state); }
  #[inline]
  fn as_any(&self) -> &dyn Any { self }
}

impl Task for Box<dyn DynTask> {
  type Output = Box<dyn DynOutput>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    self.as_ref().dyn_execute(context)
  }
  #[inline]
  fn as_dyn(&self) -> &dyn DynTask {
    self.as_ref()
  }
  #[inline]
  fn as_dyn_clone(&self) -> Box<dyn DynTask> {
    self.clone()
  }
}

impl PartialEq for dyn DynTask {
  #[inline]
  fn eq(&self, other: &dyn DynTask) -> bool { self.dyn_eq(other.as_any()) }
}

impl Eq for dyn DynTask {}

impl Hash for dyn DynTask {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.dyn_hash(state); }
}

impl Clone for Box<dyn DynTask> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(self.as_ref())
  }
}

// /// Extension trait for converting `Task`s into `dyn DynTask`s.
// pub trait TaskDynExt {
//   fn as_dyn(&self) -> &dyn DynTask;
//   fn as_dyn_clone(&self) -> Box<dyn DynTask>;
// }
// 
// impl<T: Task> TaskDynExt for T {
//   #[inline]
//   fn as_dyn(&self) -> &dyn DynTask { self as &dyn DynTask }
//   #[inline]
//   fn as_dyn_clone(&self) -> Box<dyn DynTask> { self.as_dyn().clone_box() }
// }

/// Extension trait for cloning `dyn DynTask`s.
pub trait DynTaskExt {
  fn clone_box(&self) -> Box<Self>;
}

impl DynTaskExt for dyn DynTask {
  fn clone_box(&self) -> Box<Self> {
    dyn_clone::clone_box(self)
  }
}


/// Object-safe version of [`Output`].
pub trait DynOutput: DynClone + Any + Debug {
  fn dyn_eq(&self, other: &dyn Any) -> bool;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
  fn as_box_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Output> DynOutput for T {
  #[inline]
  fn dyn_eq(&self, other: &dyn Any) -> bool {
    other.downcast_ref::<Self>().map_or(false, |o| self == o)
  }
  #[inline]
  fn as_any(&self) -> &dyn Any { self }
  #[inline]
  fn as_any_mut(&mut self) -> &mut dyn Any { self }
  #[inline]
  fn as_box_any(self: Box<Self>) -> Box<dyn Any> { self as Box<dyn Any> }
}

impl PartialEq for dyn DynOutput {
  #[inline]
  fn eq(&self, other: &dyn DynOutput) -> bool { self.dyn_eq(other.as_any()) }
}

impl Eq for dyn DynOutput {}

impl Clone for Box<dyn DynOutput> {
  fn clone(&self) -> Self {
    dyn_clone::clone_box(self.as_ref())
  }
}

/// Extension trait for cloning `dyn DynOutput`s.
pub trait DynOutputExt {
  fn clone_box(&self) -> Box<Self>;
}

impl DynOutputExt for dyn DynOutput {
  fn clone_box(&self) -> Box<Self> {
    dyn_clone::clone_box(self)
  }
}


/// Object safe version of [`Context`].
pub trait DynContext {
  fn dyn_require_task(&mut self, task: &Box<dyn DynTask>) -> Box<dyn DynOutput>;
  fn dyn_require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error>;
  fn dyn_provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error>;
}

impl Context for &mut (dyn DynContext + '_) {
  #[inline]
  fn require_task<T: Task>(&mut self, task: &T) -> T::Output {
    dbg!(&task);
    let task = task.as_dyn_clone();
    dbg!(&task);
    let output = (*self).dyn_require_task(&task);
    let output = output.as_box_any().downcast::<T::Output>().unwrap();
    *output
  }
  #[inline]
  fn require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> {
    (*self).dyn_require_file(path)
  }
  #[inline]
  fn provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error> {
    (*self).dyn_provide_file(path)
  }
}

impl<C: Context> DynContext for C {
  #[inline]
  fn dyn_require_task(&mut self, task: &Box<dyn DynTask>) -> Box<dyn DynOutput> {
    self.require_task(task)
  }
  #[inline]
  fn dyn_require_file(&mut self, path: &PathBuf) -> Result<File, std::io::Error> {
    self.require_file(path)
  }
  #[inline]
  fn dyn_provide_file(&mut self, path: &PathBuf) -> Result<(), std::io::Error> {
    self.provide_file(path)
  }
}


/// Object-safe version of [`Dependency`].
pub trait DynDependency: Debug {
  fn dyn_is_consistent(&self, context: &mut dyn DynContext) -> Result<bool, Box<dyn Error>>;
}

impl Dependency for &dyn DynDependency {
  fn is_consistent<C: Context>(&self, context: &mut C) -> Result<bool, Box<dyn Error>> {
    (*self).dyn_is_consistent(context)
  }
}

impl<D: Dependency> DynDependency for D {
  fn dyn_is_consistent(&self, mut context: &mut dyn DynContext) -> Result<bool, Box<dyn Error>> {
    self.is_consistent(&mut context)
  }
}
