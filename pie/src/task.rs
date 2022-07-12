use std::any::Any;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use dyn_clone::DynClone;

use crate::Context;

/// The unit of computation in the incremental build system.
pub trait Task: DynTask + Eq + Hash + Clone + Debug + 'static {
  /// The output this task produces when executed.
  type Output: Eq + Clone + 'static;
  /// Execute the task under `[context]` to provide incrementality, producing `[Self::Output]`
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output;
}

/// A trait object-safe version of `[Task]`, enabling tasks to be used as trait objects.
pub trait DynTask: DynClone + Debug + 'static {
  fn dyn_eq(&self, other: &dyn Any) -> bool;
  fn dyn_hash(&self, state: &mut dyn Hasher);
  fn as_any(&self) -> &dyn Any;
}


// Implement DynTask for all Tasks.

impl<T: Task> DynTask for T {
  #[inline]
  fn dyn_eq(&self, other: &dyn Any) -> bool {
    if let Some(other) = other.downcast_ref::<Self>() {
      self == other
    } else {
      false
    }
  }
  #[inline]
  fn dyn_hash(&self, mut state: &mut dyn Hasher) { self.hash(&mut state); }
  #[inline]
  fn as_any(&self) -> &dyn Any { self }
}


// Implement PartialEq/Eq/Hash/Clone for DynTask

impl PartialEq for dyn DynTask {
  #[inline]
  fn eq(&self, other: &dyn DynTask) -> bool { self.dyn_eq(other.as_any()) }
}

impl Eq for dyn DynTask {}

impl Hash for dyn DynTask {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.dyn_hash(state); }
}

dyn_clone::clone_trait_object!(DynTask);


// No-op task implementation

#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct NoopTask {}

impl Task for NoopTask {
  type Output = ();
  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output { () }
}
