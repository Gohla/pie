use std::borrow::Cow;
use std::fmt::Debug;
use std::hash::{Hash, Hasher};

use dyn_clone::DynClone;

use base::{AsAny, EqObj, HashObj};

use crate::{Key, Value, ValueEq};

#[macro_use]
pub(crate) mod base;
pub(crate) mod collection;
pub(crate) mod task;
pub(crate) mod resource;

/// Object safe [`Value`] proxy.
pub trait ValueObj: DynClone + AsAny + Debug {}
const_assert_object_safe!(dyn ValueObj);
impl<T: Value> ValueObj for T {}
impl<'a, T: Value> From<&'a T> for &'a dyn ValueObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn ValueObj }
}
impl Clone for Box<dyn ValueObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl ToOwned for dyn ValueObj {
  type Owned = Box<dyn ValueObj>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a> From<&'a dyn ValueObj> for Cow<'a, dyn ValueObj> {
  #[inline]
  fn from(value: &'a dyn ValueObj) -> Self { Cow::Borrowed(value) }
}
impl<'a> From<Box<dyn ValueObj>> for Cow<'a, dyn ValueObj> {
  #[inline]
  fn from(value: Box<dyn ValueObj>) -> Self { Cow::Owned(value) }
}

/// Object safe [`ValueEq`] proxy.
pub trait ValueEqObj: ValueObj + EqObj {}
const_assert_object_safe!(dyn ValueEqObj);
impl<T: ValueEq> ValueEqObj for T {}
impl<'a, T: ValueEq> From<&'a T> for &'a dyn ValueEqObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn ValueEqObj }
}
impl PartialEq for dyn ValueEqObj {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl Eq for dyn ValueEqObj {}
impl PartialEq<dyn ValueEqObj> for Box<dyn ValueEqObj> {
  #[inline]
  fn eq(&self, other: &dyn ValueEqObj) -> bool { self.as_ref().eq_any(other.as_any()) }
}
impl Clone for Box<dyn ValueEqObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl ToOwned for dyn ValueEqObj {
  type Owned = Box<dyn ValueEqObj>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a> From<&'a dyn ValueEqObj> for Cow<'a, dyn ValueEqObj> {
  #[inline]
  fn from(value: &'a dyn ValueEqObj) -> Self { Cow::Borrowed(value) }
}
impl<'a> From<Box<dyn ValueEqObj>> for Cow<'a, dyn ValueEqObj> {
  #[inline]
  fn from(value: Box<dyn ValueEqObj>) -> Self { Cow::Owned(value) }
}

/// Object safe [`Key`] proxy.
pub trait KeyObj: DynClone + EqObj + HashObj + AsAny + Debug {}
const_assert_object_safe!(dyn KeyObj);
impl<T: Key> KeyObj for T {}
impl<'a, T: Key> From<&'a T> for &'a dyn KeyObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn KeyObj }
}
impl PartialEq for dyn KeyObj {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl Eq for dyn KeyObj {}
impl PartialEq<dyn KeyObj> for Box<dyn KeyObj> {
  #[inline]
  fn eq(&self, other: &dyn KeyObj) -> bool { self.as_ref().eq_any(other.as_any()) }
}
impl Hash for dyn KeyObj {
  #[inline]
  fn hash<H: Hasher>(&self, state: &mut H) { self.hash_obj(state); }
}
impl Clone for Box<dyn KeyObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl ToOwned for dyn KeyObj {
  type Owned = Box<dyn KeyObj>;
  #[inline]
  fn to_owned(&self) -> Self::Owned { dyn_clone::clone_box(self) }
}
impl<'a> From<&'a dyn KeyObj> for Cow<'a, dyn KeyObj> {
  #[inline]
  fn from(value: &'a dyn KeyObj) -> Self { Cow::Borrowed(value) }
}
impl<'a> From<Box<dyn KeyObj>> for Cow<'a, dyn KeyObj> {
  #[inline]
  fn from(value: Box<dyn KeyObj>) -> Self { Cow::Owned(value) }
}

#[cfg(test)]
mod tests {
  use std::collections::hash_map::DefaultHasher;

  use crate::{Context, Task};

  use super::*;

  #[test]
  fn test_val_obj() {
    #[derive(Clone, Debug)]
    #[allow(dead_code)]
    struct ATaskOutput(usize);

    let output_1 = ATaskOutput(1);
    let output_2 = ATaskOutput(2);
    let val_1: Box<dyn ValueObj> = Box::new(output_1.clone());
    let val_2: Box<dyn ValueObj> = Box::new(output_2.clone());

    macro_rules! assert_debug_eq {
      ($left:expr, $right:expr $(,)?) => {
        assert_eq!(format!("{:?}", $left), format!("{:?}", $right))
      };
    }
    macro_rules! assert_debug_ne {
      ($left:expr, $right:expr $(,)?) => {
        assert_ne!(format!("{:?}", $left), format!("{:?}", $right))
      };
    }

    // Eq, PartialEq - through Debug
    assert_debug_eq!(val_1, val_1);
    assert_debug_ne!(val_1, val_2);
    assert_debug_eq!(&val_1, &val_1);
    assert_debug_ne!(&val_1, &val_2);
    assert_debug_eq!(val_1.as_ref(), val_1.as_ref());
    assert_debug_ne!(val_1.as_ref(), val_2.as_ref());
    // AsAny - through Debug
    assert_debug_eq!(val_1.as_ref().as_any().downcast_ref::<ATaskOutput>(), Some(&output_1));
    assert_debug_ne!(val_1.as_ref().as_any().downcast_ref::<ATaskOutput>(), Some(&output_2));
    assert_debug_ne!(val_2.as_ref().as_any().downcast_ref::<ATaskOutput>(), Some(&output_1));
    assert_debug_eq!(*val_1.clone().into_box_any().downcast::<ATaskOutput>().unwrap(), output_1.clone());
    assert_debug_ne!(*val_1.clone().into_box_any().downcast::<ATaskOutput>().unwrap(), output_2.clone());
    assert_debug_ne!(*val_2.clone().into_box_any().downcast::<ATaskOutput>().unwrap(), output_1.clone());
    // Clone - through Debug
    assert_debug_eq!(val_1, val_1.clone());
    assert_debug_ne!(val_1, val_2.clone());
    assert_debug_ne!(val_2, val_1.clone());
    assert_debug_eq!(val_1, val_1.to_owned());
    assert_debug_ne!(val_1, val_2.to_owned());
    assert_debug_ne!(val_2, val_1.to_owned());
    // Cow - through Debug
    assert_debug_eq!(Cow::from(val_1.as_ref()), Cow::from(val_1.as_ref()));
    assert_debug_ne!(Cow::from(val_1.as_ref()), Cow::from(val_2.as_ref()));
    assert_debug_eq!(Cow::from(val_1.as_ref()).into_owned(), val_1);
    assert_debug_ne!(Cow::from(val_1.as_ref()).into_owned(), val_2);
    assert_debug_ne!(Cow::from(val_2.as_ref()).into_owned(), val_1);
    // Debug
    assert_eq!(format!("{:?}", val_1), format!("{:?}", output_1));
    assert_ne!(format!("{:?}", val_1), format!("{:?}", output_2));
  }

  #[test]
  fn test_key_obj() {
    #[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
    struct ToLowerCase(String);
    impl Task for ToLowerCase {
      type Output = String;
      fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
        self.0.to_ascii_lowercase()
      }
    }

    let task_a = ToLowerCase("A".to_string());
    let task_b = ToLowerCase("B".to_string());
    let key_a: Box<dyn KeyObj> = Box::new(task_a.clone());
    let key_b: Box<dyn KeyObj> = Box::new(task_b.clone());

    // Eq, PartialEq
    assert_eq!(key_a, key_a);
    assert_ne!(key_a, key_b);
    assert_eq!(&key_a, &key_a);
    assert_ne!(&key_a, &key_b);
    assert_eq!(key_a.as_ref(), key_a.as_ref());
    assert_ne!(key_a.as_ref(), key_b.as_ref());
    // Hash
    let mut hasher = DefaultHasher::new();
    assert_eq!(key_a.hash(&mut hasher), task_a.hash(&mut hasher));
    // AsAny
    // Note: key is `Box<dyn KeyObj>` which also implements `AsAny`, but would fail to downcast. Need to first call
    // `as_ref` to convert `Box<dyn KeyObj>` into `&dyn KeyObj` which succeeds the downcast.
    assert_eq!(key_a.as_ref().as_any().downcast_ref::<ToLowerCase>(), Some(&task_a));
    assert_ne!(key_a.as_ref().as_any().downcast_ref::<ToLowerCase>(), Some(&task_b));
    assert_ne!(key_b.as_ref().as_any().downcast_ref::<ToLowerCase>(), Some(&task_a));
    assert_eq!(*key_a.clone().into_box_any().downcast::<ToLowerCase>().unwrap(), task_a.clone());
    assert_ne!(*key_a.clone().into_box_any().downcast::<ToLowerCase>().unwrap(), task_b.clone());
    assert_ne!(*key_b.clone().into_box_any().downcast::<ToLowerCase>().unwrap(), task_a.clone());
    // Clone
    assert_eq!(key_a, key_a.clone());
    assert_ne!(key_a, key_b.clone());
    assert_ne!(key_b, key_a.clone());
    assert_eq!(key_a, key_a.to_owned());
    assert_ne!(key_a, key_b.to_owned());
    assert_ne!(key_b, key_a.to_owned());
    // Cow
    assert_eq!(Cow::from(key_a.as_ref()), Cow::from(key_a.as_ref()));
    assert_ne!(Cow::from(key_a.as_ref()), Cow::from(key_b.as_ref()));
    assert_eq!(Cow::from(key_a.as_ref()).into_owned(), key_a);
    assert_ne!(Cow::from(key_a.as_ref()).into_owned(), key_b);
    assert_ne!(Cow::from(key_b.as_ref()).into_owned(), key_a);
    // Debug
    assert_eq!(format!("{:?}", key_a), format!("{:?}", task_a));
    assert_ne!(format!("{:?}", key_a), format!("{:?}", task_b));
  }
}
