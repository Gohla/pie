use std::any::Any;
use std::fmt::Debug;

use dyn_clone::DynClone;

/// Trait alias for task outputs.
pub trait Output: Eq + Clone + Any + Debug {}

impl<T: Eq + Clone + Any + Debug> Output for T {}


/// Object-safe version of [`Output`].
pub trait DynOutput: DynClone + Any + Debug + 'static {
  fn dyn_eq(&self, other: &dyn Any) -> bool;
  fn as_any(&self) -> &dyn Any;
  fn as_any_mut(&mut self) -> &mut dyn Any;
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


// Extension trait to enable calling `clone` on `dyn DynOutput`.
pub trait DynOutputExt {
  fn clone(&self) -> Box<Self>;
}

impl DynOutputExt for dyn DynOutput {
  fn clone(&self) -> Box<Self> {
    dyn_clone::clone_box(self)
  }
}
