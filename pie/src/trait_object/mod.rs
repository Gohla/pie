use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use dyn_clone::DynClone;

use base::{AsAny, EqObj, HashObj};

use crate::{KeyBounds, ValueBounds};
use crate::trait_object::base::CloneBox;

pub(crate) mod collection;
pub(crate) mod base;
pub(crate) mod task;

/// Object safe [`KeyBounds`] proxy that can be cloned, equality compared, hashed, converted to [`Any`], and debug
/// formatted.
pub trait KeyObj: DynClone + EqObj + HashObj + AsAny + Debug {}
impl<T: KeyBounds> KeyObj for T {}
impl<'a, T: KeyBounds> From<&'a T> for &'a dyn KeyObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn KeyObj }
}
impl PartialEq for dyn KeyObj {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl Eq for dyn KeyObj {}
impl Hash for dyn KeyObj {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl Clone for Box<dyn KeyObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl<T: KeyBounds> CloneBox<dyn KeyObj> for T {
  #[inline]
  fn clone_box(&self) -> Box<dyn KeyObj> { dyn_clone::clone_box(self) }
}
impl CloneBox<dyn KeyObj> for dyn KeyObj {
  #[inline]
  fn clone_box(&self) -> Box<dyn KeyObj> { dyn_clone::clone_box(self) }
}

/// Object safe [`ValueBounds`] proxy that can be cloned, converted to [`Any`], and debug formatted.
pub trait ValueObj: DynClone + AsAny + Debug {}
impl<T: ValueBounds> ValueObj for T {}
impl<'a, T: ValueBounds> From<&'a T> for &'a dyn ValueObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn ValueObj }
}
impl Clone for Box<dyn ValueObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl<T: ValueBounds> CloneBox<dyn ValueObj> for T {
  #[inline]
  fn clone_box(&self) -> Box<dyn ValueObj> { dyn_clone::clone_box(self) }
}
impl CloneBox<dyn ValueObj> for dyn ValueObj {
  #[inline]
  fn clone_box(&self) -> Box<dyn ValueObj> { dyn_clone::clone_box(self) }
}
