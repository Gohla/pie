use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use crate::{OutputChecker, Task};
use crate::context::bottom_up::BottomUpContext;
use crate::context::top_down::TopDownContext;
use crate::trait_object::{KeyObj, ValueObj};

/// Internal object safe [`Task`] proxy. Has execute methods for concrete [`Context`] implementations, instead of a
/// generic method, due to object safety.
pub trait TaskObj: KeyObj {
  fn as_key_obj(&self) -> &dyn KeyObj;
  fn execute_top_down(&self, context: &mut TopDownContext) -> Box<dyn ValueObj>;
  fn execute_bottom_up(&self, context: &mut BottomUpContext) -> Box<dyn ValueObj>;
}
const_assert_object_safe!(dyn TaskObj);
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
impl PartialEq<dyn TaskObj> for Box<dyn TaskObj> {
  #[inline]
  fn eq(&self, other: &dyn TaskObj) -> bool { self.as_ref().eq_any(other.as_any()) }
}
impl Hash for dyn TaskObj {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl Clone for Box<dyn TaskObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl ToOwned for dyn TaskObj {
  type Owned = Box<dyn TaskObj>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a> From<&'a dyn TaskObj> for Cow<'a, dyn TaskObj> {
  #[inline]
  fn from(value: &'a dyn TaskObj) -> Self { Cow::Borrowed(value) }
}
impl<'a> From<Box<dyn TaskObj>> for Cow<'a, dyn TaskObj> {
  #[inline]
  fn from(value: Box<dyn TaskObj>) -> Self { Cow::Owned(value) }
}


/// Internal object safe [`OutputChecker`] proxy.
pub trait OutputCheckerObj<O>: KeyObj {
  fn stamp_obj(&self, output: &O) -> Box<dyn ValueObj>;
  fn check_obj<'i>(&'i self, output: &'i O, stamp: &'i dyn ValueObj) -> Option<Box<dyn Debug + 'i>>;
}
const_assert_object_safe!(dyn OutputCheckerObj<()>);
impl<O, C: OutputChecker<O>> OutputCheckerObj<O> for C {
  #[inline]
  fn stamp_obj(&self, output: &O) -> Box<dyn ValueObj> {
    Box::new(self.stamp(output))
  }
  #[inline]
  fn check_obj<'i>(&'i self, output: &'i O, stamp: &'i dyn ValueObj) -> Option<Box<dyn Debug + 'i>> {
    let stamp_typed = stamp.as_any().downcast_ref::<C::Stamp>()
      .expect("BUG: non-matching stamp type");
    self.check(output, stamp_typed)//.map(|i| Box::new(i) as Box<dyn Debug>)
  }
}
impl<'a, O, T: OutputChecker<O>> From<&'a T> for &'a dyn OutputCheckerObj<O> {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn OutputCheckerObj<O> }
}
impl<O: 'static> PartialEq for dyn OutputCheckerObj<O> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl<O: 'static> Eq for dyn OutputCheckerObj<O> {}
impl<O: 'static> PartialEq<dyn OutputCheckerObj<O>> for Box<dyn OutputCheckerObj<O>> {
  #[inline]
  fn eq(&self, other: &dyn OutputCheckerObj<O>) -> bool { self.as_ref().eq_any(other.as_any()) }
}
impl<O> Hash for dyn OutputCheckerObj<O> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl<O> Clone for Box<dyn OutputCheckerObj<O>> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl<O> ToOwned for dyn OutputCheckerObj<O> {
  type Owned = Box<dyn OutputCheckerObj<O>>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a, O> From<&'a dyn OutputCheckerObj<O>> for Cow<'a, dyn OutputCheckerObj<O>> {
  #[inline]
  fn from(value: &'a dyn OutputCheckerObj<O>) -> Self { Cow::Borrowed(value) }
}
impl<'a, O> From<Box<dyn OutputCheckerObj<O>>> for Cow<'a, dyn OutputCheckerObj<O>> {
  #[inline]
  fn from(value: Box<dyn OutputCheckerObj<O>>) -> Self { Cow::Owned(value) }
}


#[cfg(test)]
mod tests {
  use crate::task::EqualsChecker;

  use super::*;

  #[test]
  fn test_output_checker_obj() {
    let output_1 = 1usize;
    let output_2 = 2usize;

    let equals_checker = EqualsChecker;
    let output_checker_obj: Box<dyn OutputCheckerObj<usize>> = Box::new(equals_checker);
    let stamp_obj_1 = output_checker_obj.stamp_obj(&output_1);
    let stamp_obj_2 = output_checker_obj.stamp_obj(&output_2);
    assert!(output_checker_obj.check_obj(&output_1, stamp_obj_1.as_ref()).is_none());
    assert!(output_checker_obj.check_obj(&output_2, stamp_obj_2.as_ref()).is_none());
    assert!(output_checker_obj.check_obj(&output_1, stamp_obj_2.as_ref()).is_some());
    assert!(output_checker_obj.check_obj(&output_2, stamp_obj_1.as_ref()).is_some());

    let stamp_1 = equals_checker.stamp(&output_1);
    let stamp_2 = equals_checker.stamp(&output_2);
    assert!(output_checker_obj.check_obj(&output_1, &stamp_1).is_none());
    assert!(output_checker_obj.check_obj(&output_2, &stamp_2).is_none());
    assert!(output_checker_obj.check_obj(&output_1, &stamp_2).is_some());
    assert!(output_checker_obj.check_obj(&output_2, &stamp_1).is_some());
  }
}
