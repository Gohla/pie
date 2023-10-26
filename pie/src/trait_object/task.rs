use std::hash::{Hash, Hasher};

use crate::context::bottom_up::BottomUpContext;
use crate::context::top_down::TopDownContext;
use crate::Task;
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::base::CloneBox;

pub trait TaskObj: KeyObj {
  fn as_key_obj(&self) -> &dyn KeyObj;
  fn execute_top_down(&self, context: &mut TopDownContext) -> Box<dyn ValueObj>;
  fn execute_bottom_up(&self, context: &mut BottomUpContext) -> Box<dyn ValueObj>;
}
impl<T: Task> TaskObj for T {
  #[inline]
  fn as_key_obj(&self) -> &dyn KeyObj { self as &dyn KeyObj }
  #[inline]
  fn execute_top_down(&self, context: &mut TopDownContext) -> Box<dyn ValueObj> {
    Box::new(self.execute(context))
  }
  #[inline]
  fn execute_bottom_up(&self, context: &mut BottomUpContext) -> Box<dyn ValueObj> {
    Box::new(self.execute(context))
  }
}

impl<'a, T: Task> From<&'a T> for &'a dyn TaskObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn TaskObj }
}
impl PartialEq for dyn TaskObj {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl Eq for dyn TaskObj {}
impl Hash for dyn TaskObj {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl Clone for Box<dyn TaskObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl<T: Task> CloneBox<dyn TaskObj> for T {
  #[inline]
  fn clone_box(&self) -> Box<dyn TaskObj> { dyn_clone::clone_box(self) }
}
impl CloneBox<dyn TaskObj> for dyn TaskObj {
  #[inline]
  fn clone_box(&self) -> Box<dyn TaskObj> { dyn_clone::clone_box(self) }
}
