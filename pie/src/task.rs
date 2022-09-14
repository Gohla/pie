use std::any::Any;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use dyn_clone::DynClone;

use crate::Context;
use crate::output::Output;

/// The unit of computation in the incremental build system.
pub trait Task: Eq + Hash + Clone + Any + Debug {
  /// The type of output this task produces when executed. Must implement [`Eq`], [`Clone`], and either not contain any 
  /// references, or only `'static` references.
  type Output: Output;
  /// Execute the task, with `context` providing a means to specify dependencies, producing an instance of 
  /// `Self::Output`.
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;

  #[inline]
  fn as_dyn(&self) -> &dyn DynTask { self as &dyn DynTask }
  #[inline]
  fn as_dyn_clone(&self) -> Box<dyn DynTask> { self.as_dyn().clone() }
}


/// Object-safe version of [`Task`], enabling tasks to be used as trait objects.
pub trait DynTask: DynClone + Any + Debug + 'static {
  fn dyn_eq(&self, other: &dyn Any) -> bool;
  fn dyn_hash(&self, state: &mut dyn Hasher);
  fn as_any(&self) -> &dyn Any;
}

impl<T: Task> DynTask for T {
  #[inline]
  fn dyn_eq(&self, other: &dyn Any) -> bool {
    other.downcast_ref::<Self>().map_or(false, |o| self == o)
  }
  #[inline]
  fn dyn_hash(&self, mut state: &mut dyn Hasher) { self.hash(&mut state); }
  #[inline]
  fn as_any(&self) -> &dyn Any { self }
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
    dyn_clone::clone_box(&**self)
  }
}


/// Extension trait to enable calling `clone` on `dyn DynTask`.
pub trait DynTaskExt {
  fn clone(&self) -> Box<Self>;
}

impl DynTaskExt for dyn DynTask {
  fn clone(&self) -> Box<Self> {
    dyn_clone::clone_box(self)
  }
}


/// Task that does nothing and returns `()`.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct NoopTask {}

impl Task for NoopTask {
  type Output = ();
  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output { () }
}
