use std::any::Any;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt::Debug;
use std::hash::Hash;

use dyn_clone::DynClone;

use crate::{Key, Resource, ResourceChecker, ResourceState};
use crate::trait_object::base::{AsAny, EqObj};
use crate::trait_object::KeyObj;

/// Key of globally shared [hash maps](HashMap) from [`Self`] to [`Self::Value`].
pub trait MapKey: Key {
  type Value;
}
impl<K: MapKey> Resource for K {
  type Reader<'rc> = Option<&'rc K::Value>;
  type Writer<'r> = MapWriter<'r, K>;
  type Error = Infallible;
  #[inline]
  fn read<'rc, C: ResourceState<Self>>(&self, state: &'rc mut C) -> Result<Self::Reader<'rc>, Self::Error> {
    let map = state.get_global_map();
    let value = map.get(&self);
    Ok(value)
  }
  #[inline]
  fn write<'r, C: ResourceState<Self>>(&'r self, state: &'r mut C) -> Result<Self::Writer<'r>, Self::Error> {
    let map = state.get_global_map_mut();
    let writer = MapWriter { map, key: self };
    Ok(writer)
  }
}

/// Writer for a specific `key` into globally shared hash `map`.
pub struct MapWriter<'a, K: MapKey> {
  map: &'a mut HashMap<K, K::Value>,
  key: &'a K,
}
impl<K: MapKey> MapWriter<'_, K> {
  /// Gets the value, returning `Some(&value)` if it exists, `None` otherwise.
  #[inline]
  pub fn get(&self) -> Option<&K::Value> { self.map.get(self.key) }
  /// Gets the mutable value, returning `Some(&mut value)` if it exists, `None` otherwise.
  #[inline]
  pub fn get_mut(&mut self) -> Option<&mut K::Value> { self.map.get_mut(self.key) }
  /// Inserts `value`, returning `Some(previous_value)` if there was a previous value, `None` otherwise.
  #[inline]
  pub fn insert(&mut self, value: K::Value) -> Option<K::Value> where K: Clone {
    self.map.insert(self.key.clone(), value)
  }
  /// Gets the entry for in-place manipulation.
  #[inline]
  pub fn entry(&mut self) -> Entry<K, K::Value> where K: Clone { self.map.entry(self.key.clone()) }
}

/// Convenience trait for getting global hash maps for key type `K`.
pub trait GetGlobalMap<K: MapKey> {
  /// Gets the global hash map for key type `K`.
  fn get_global_map(&mut self) -> &HashMap<K, K::Value>;
  /// Gets the mutable global hash map for key type `K`.
  fn get_global_map_mut(&mut self) -> &mut HashMap<K, K::Value>;
}
impl<K: MapKey, RS: ResourceState<K>> GetGlobalMap<K> for RS {
  #[inline]
  fn get_global_map(&mut self) -> &HashMap<K, K::Value> {
    self.get_or_set_default::<HashMap<K, K::Value>>()
  }
  #[inline]
  fn get_global_map_mut(&mut self) -> &mut HashMap<K, K::Value> {
    self.get_or_set_default_mut::<HashMap<K, K::Value>>()
  }
}


/// Hash map [resource checker](ResourceChecker) that marks hash map dependencies as consistent when the value
/// corresponding to its key is equal.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct MapEqualsChecker;
impl<K: MapKey> ResourceChecker<K> for MapEqualsChecker where
  K::Value: Clone + Eq + Debug
{
  type Stamp = Option<K::Value>;
  type Error = Infallible;
  #[inline]
  fn stamp<RS: ResourceState<K>>(&self, key: &K, state: &mut RS) -> Result<Self::Stamp, Self::Error> {
    let value = key.read(state)?.map(|v| v.clone());
    Ok(value)
  }
  #[inline]
  fn stamp_reader(&self, _key: &K, value: &mut Option<&K::Value>) -> Result<Self::Stamp, Self::Error> {
    let value = value.map(|v| v.clone());
    Ok(value)
  }
  #[inline]
  fn stamp_writer(&self, _key: &K, writer: MapWriter<'_, K>) -> Result<Self::Stamp, Self::Error> {
    let value = writer.get().map(|v| v.clone());
    Ok(value)
  }

  #[inline]
  fn check<RS: ResourceState<K>>(
    &self,
    key: &K,
    state: &mut RS,
    stamp: &Self::Stamp,
  ) -> Result<Option<impl Debug>, Self::Error> {
    let value = key.read(state)?;
    let inconsistency = if value != stamp.as_ref() {
      Some(value)
    } else {
      None
    };
    Ok(inconsistency)
  }
  #[inline]
  fn wrap_error(&self, error: Infallible) -> Self::Error { error }
}


/// [`MapKey`] from [`K`] to [`Box<dyn MapValueObj>`].
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct MapKeyToObj<K>(pub K);
impl<K: Key> MapKey for MapKeyToObj<K> {
  type Value = Box<dyn MapValueObj>;
}
impl<K> MapKeyToObj<K> {
  #[inline]
  pub fn new(key: K) -> Self { Self(key) }
}
impl<K> From<K> for MapKeyToObj<K> {
  #[inline]
  fn from(value: K) -> Self { Self::new(value) }
}

/// [`MapKey`] from [`Box<dyn KeyObj>`] to [`Box<dyn MapValueObj>`].
#[derive(Clone, Eq, Hash, Debug)]
#[repr(transparent)]
pub struct MapKeyObjToObj(pub Box<dyn KeyObj>);
impl MapKey for MapKeyObjToObj {
  type Value = Box<dyn MapValueObj>;
}
impl MapKeyObjToObj {
  #[inline]
  pub fn new(key: Box<dyn KeyObj>) -> Self { Self(key) }
  #[inline]
  pub fn from<K: Clone + Eq + Hash + Any + Debug>(key: K) -> Self { Self(Box::new(key)) }
}
impl<K: Clone + Eq + Hash + Any + Debug> From<Box<K>> for MapKeyObjToObj {
  #[inline]
  fn from(value: Box<K>) -> Self { Self::new(value) }
}
impl From<Box<dyn KeyObj>> for MapKeyObjToObj {
  #[inline]
  fn from(value: Box<dyn KeyObj>) -> Self { Self::new(value) }
}
impl PartialEq for MapKeyObjToObj {
  // Manual impl because derive is borked.
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.0.as_ref().eq(other.0.as_ref()) }
}

/// Object safe value that can be cloned, equality compared, converted to [`Any`], and debug formatted.
pub trait MapValueObj: DynClone + EqObj + AsAny + Debug {}
impl<T: Clone + Eq + Any + Debug> MapValueObj for T {}
impl<'a, T: Clone + Eq + Any + Debug> From<&'a T> for &'a dyn MapValueObj {
  #[inline]
  fn from(value: &'a T) -> Self { value as &dyn MapValueObj }
}
impl Clone for Box<dyn MapValueObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}
impl PartialEq for dyn MapValueObj {
  #[inline]
  fn eq(&self, other: &Self) -> bool { self.eq_any(other.as_any()) }
}
impl Eq for dyn MapValueObj {}
