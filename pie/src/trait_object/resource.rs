use std::borrow::Cow;
use std::error::Error;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use crate::{Resource, ResourceChecker};
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::collection::TypeToAnyMap;

/// Internal object safe [`ResourceChecker`] proxy.
pub trait ResourceCheckerObj<R>: KeyObj {
  fn check_obj<'i>(
    &'i self,
    resource: &'i R,
    state: &'i mut TypeToAnyMap,
    stamp: &'i dyn ValueObj,
  ) -> Result<Option<Box<dyn Debug + 'i>>, Box<dyn Error>>;

  fn as_key_obj(&self) -> &dyn KeyObj;
}
const_assert_object_safe!(dyn ResourceCheckerObj<()>);
impl<R: Resource, C: ResourceChecker<R>> ResourceCheckerObj<R> for C {
  fn check_obj<'i>(
    &'i self,
    resource: &'i R,
    state: &'i mut TypeToAnyMap,
    stamp: &'i dyn ValueObj,
  ) -> Result<Option<Box<dyn Debug + 'i>>, Box<dyn Error>> {
    let stamp_typed = stamp.as_any().downcast_ref::<C::Stamp>()
      .expect("BUG: non-matching stamp type");
    let inconsistency = self.check(resource, state, stamp_typed)
      .map(|o| o.map(|i| Box::new(i) as Box<dyn Debug>))?;
    Ok(inconsistency)
  }

  #[inline]
  fn as_key_obj(&self) -> &dyn KeyObj {
    self as &dyn KeyObj
  }
}
impl<'a, R: Resource, T: ResourceChecker<R>> From<&'a T> for &'a dyn ResourceCheckerObj<R> {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn ResourceCheckerObj<R> }
}
impl<R: 'static> PartialEq for dyn ResourceCheckerObj<R> {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl<R: 'static> Eq for dyn ResourceCheckerObj<R> {}
impl<R: 'static> PartialEq<dyn ResourceCheckerObj<R>> for Box<dyn ResourceCheckerObj<R>> {
  #[inline]
  fn eq(&self, other: &dyn ResourceCheckerObj<R>) -> bool { self.as_ref().eq_any(other.as_any()) }
}
impl<R> Hash for dyn ResourceCheckerObj<R> {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl<R> Clone for Box<dyn ResourceCheckerObj<R>> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl<R> ToOwned for dyn ResourceCheckerObj<R> {
  type Owned = Box<dyn ResourceCheckerObj<R>>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a, R> From<&'a dyn ResourceCheckerObj<R>> for Cow<'a, dyn ResourceCheckerObj<R>> {
  #[inline]
  fn from(value: &'a dyn ResourceCheckerObj<R>) -> Self { Cow::Borrowed(value) }
}
impl<'a, R> From<Box<dyn ResourceCheckerObj<R>>> for Cow<'a, dyn ResourceCheckerObj<R>> {
  #[inline]
  fn from(value: Box<dyn ResourceCheckerObj<R>>) -> Self { Cow::Owned(value) }
}


#[cfg(test)]
mod tests {
  use std::convert::Infallible;

  use assert_matches::assert_matches;

  use crate::resource::map::{GetGlobalMap, MapEqualsChecker, MapKey};

  use super::*;

  impl MapKey for &'static str {
    type Value = usize;
  }

  #[test]
  fn test_output_checker_obj() -> Result<(), Infallible> {
    let key_1 = "key 1";
    let value_1 = 1;
    let key_2 = "key 2";

    let map_equals_checker = MapEqualsChecker;
    let resource_checker_obj: Box<dyn ResourceCheckerObj<&'static str>> = Box::new(map_equals_checker);
    let mut resource_state = TypeToAnyMap::default();
    resource_state.get_global_map_mut().insert(key_1, value_1);
    let stamp_1 = map_equals_checker.stamp(&key_1, &mut resource_state)?;
    let stamp_2 = map_equals_checker.stamp(&key_2, &mut resource_state)?;
    assert_matches!(resource_checker_obj.check_obj(&key_1, &mut resource_state, &stamp_1), Ok(None));
    assert_matches!(resource_checker_obj.check_obj(&key_2, &mut resource_state, &stamp_2), Ok(None));
    assert_matches!(resource_checker_obj.check_obj(&key_1, &mut resource_state, &stamp_2), Ok(Some(i)) if format!("{:?}", i) == "Some(1)");
    assert_matches!(resource_checker_obj.check_obj(&key_2, &mut resource_state, &stamp_1), Ok(Some(i)) if format!("{:?}", i) == "None");
    Ok(())
  }
}
