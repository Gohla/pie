use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use crate::{OutputChecker, Task};
use crate::context::bottom_up::BottomUpContext;
use crate::serialize::{MaybeErasedSerialize, MaybeIdObj};
use crate::trait_object::{KeyObj, ValueObj};

/// Internal object safe [`Task`] proxy with type-erased output. Has execute methods for concrete [`Context`]
/// implementations, instead of a generic method, due to object safety.
pub trait TaskErasedObj: KeyObj + MaybeErasedSerialize + MaybeIdObj {
  fn execute_bottom_up(&self, context: &mut BottomUpContext) -> Box<dyn ValueObj>;

  fn as_key_obj(&self) -> &dyn KeyObj;
}
const_assert_object_safe!(dyn TaskErasedObj);

impl<T: Task> TaskErasedObj for T {
  #[inline]
  fn execute_bottom_up(&self, context: &mut BottomUpContext) -> Box<dyn ValueObj> {
    Box::new(self.execute(context))
  }

  #[inline]
  fn as_key_obj(&self) -> &dyn KeyObj { self as &dyn KeyObj }
}

impl<'a, T: Task> From<&'a T> for &'a dyn TaskErasedObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn TaskErasedObj }
}
impl<T: Task> From<T> for Box<dyn TaskErasedObj> {
  #[inline]
  fn from(value: T) -> Self { Box::new(value) }
}
impl PartialEq for dyn TaskErasedObj {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl Eq for dyn TaskErasedObj {}
impl PartialEq<dyn TaskErasedObj> for Box<dyn TaskErasedObj> {
  #[inline]
  fn eq(&self, other: &dyn TaskErasedObj) -> bool { self.as_ref().eq_any(other.as_any()) }
}
impl Hash for dyn TaskErasedObj {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl Clone for Box<dyn TaskErasedObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl ToOwned for dyn TaskErasedObj {
  type Owned = Box<dyn TaskErasedObj>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a> From<&'a dyn TaskErasedObj> for Cow<'a, dyn TaskErasedObj> {
  #[inline]
  fn from(value: &'a dyn TaskErasedObj) -> Self { Cow::Borrowed(value) }
}
impl<'a> From<Box<dyn TaskErasedObj>> for Cow<'a, dyn TaskErasedObj> {
  #[inline]
  fn from(value: Box<dyn TaskErasedObj>) -> Self { Cow::Owned(value) }
}


/// Internal object safe [`OutputChecker`] proxy with type-erased stamp.
pub trait OutputCheckerObj<O>: KeyObj {
  fn check_obj<'i>(&'i self, output: &'i O, stamp: &'i dyn ValueObj) -> Option<Box<dyn Debug + 'i>>;

  fn as_key_obj(&self) -> &dyn KeyObj;
}
const_assert_object_safe!(dyn OutputCheckerObj<()>);
impl<O, C: OutputChecker<O>> OutputCheckerObj<O> for C {
  #[inline]
  fn check_obj<'i>(&'i self, output: &'i O, stamp: &'i dyn ValueObj) -> Option<Box<dyn Debug + 'i>> {
    let stamp_typed = stamp.as_any().downcast_ref::<C::Stamp>()
      .expect("BUG: non-matching stamp type");
    self.check(output, stamp_typed)
      .map(|i| Box::new(i) as Box<dyn Debug>)
  }

  #[inline]
  fn as_key_obj(&self) -> &dyn KeyObj { self as &dyn KeyObj }
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
  use assert_matches::assert_matches;

  use crate::task::EqualsChecker;

  use super::*;

  #[test]
  fn test_output_checker_obj() {
    let equals_checker = EqualsChecker;
    let output_checker_obj: Box<dyn OutputCheckerObj<usize>> = Box::new(equals_checker);
    let output_1 = 1usize;
    let output_2 = 2usize;
    let stamp_1 = equals_checker.stamp(&output_1);
    let stamp_2 = equals_checker.stamp(&output_2);
    assert_matches!(output_checker_obj.check_obj(&output_1, &stamp_1), None);
    assert_matches!(output_checker_obj.check_obj(&output_2, &stamp_2), None);
    assert_matches!(output_checker_obj.check_obj(&output_1, &stamp_2), Some(i) if format!("{:?}", i) == "1");
    assert_matches!(output_checker_obj.check_obj(&output_2, &stamp_1), Some(i) if format!("{:?}", i) == "2");
  }
}
