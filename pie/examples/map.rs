use std::fmt::Debug;
use std::hash::Hash;

use pie::{Context, Pie, Task};
use pie::resource::file::FsError;
use pie::resource::map::{GetGlobalMap, MapEqualsChecker, MapKey, MapKeyObjToObj, MapKeyToObj};
use pie::task::{AlwaysConsistent, EqualsChecker};
use pie::tracker::writing::WritingTracker;

/// Task that returns the value at `key` from the global map for type `K`.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct ReadMap<K, T> {
  key: K,
  origin_task: Option<T>,
}

impl<K> ReadMap<K, ()> {
  pub fn new(key: impl Into<K>) -> Self {
    Self {
      key: key.into(),
      origin_task: None,
    }
  }
}

impl<K, T> ReadMap<K, T> {
  pub fn with_origin(key: impl Into<K>, origin_task: T) -> Self {
    Self {
      key: key.into(),
      origin_task: Some(origin_task),
    }
  }
}

impl<K: MapKey, T: Task> Task for ReadMap<K, T> where
  K::Value: Clone + Eq + Debug
{
  type Output = Option<K::Value>;
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    if let Some(origin_task) = &self.origin_task {
      context.require(origin_task, AlwaysConsistent);
    }
    context.read(&self.key, MapEqualsChecker).unwrap().cloned()
  }
}


/// Task that gets the value to write by requiring `value_provider`, then writes that to the global map at `key`.
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct WriteMap<K, T> {
  key: K,
  value_provider: T,
}

impl<K, T> WriteMap<K, T> {
  pub fn new(key: impl Into<K>, value_provider: T) -> Self {
    Self {
      value_provider,
      key: key.into(),
    }
  }
}

impl<K: MapKey, T: Task<Output=K::Value>> Task for WriteMap<K, T> where
  K::Value: Clone + Eq + Debug
{
  type Output = ();
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let value = context.require(&self.value_provider, EqualsChecker);
    context.write(&self.key, MapEqualsChecker, |writer| {
      writer.insert(value);
      Ok(())
    }).unwrap();
  }
}

/// Task that always returns "constant".
#[derive(Clone, Eq, PartialEq, Hash, Debug)]
struct Constant;
impl Task for Constant {
  type Output = &'static str;
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
    "constant"
  }
}


/// Key for use in maps, wrapping `&'static str`.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
struct Key(&'static str);
impl From<&'static str> for Key {
  fn from(value: &'static str) -> Self {
    Key(value)
  }
}

// Implement `MapKey` for `Key` type, resulting in a map from `&'static str` to `&'static str`.
impl MapKey for Key {
  type Value = &'static str;
}


fn main() -> Result<(), FsError> {
  let mut pie = Pie::with_tracker(WritingTracker::with_stdout());

  {
    println!("Typed key to value:");


    pie.resource_state_mut().get_global_map_mut().insert(Key("manual key"), "manual value");
    let read = ReadMap::<Key, _>::new("manual key");
    pie.new_session().require(&read);

    // Create `read_from_written` task that reads from "write-read key" which is written to by `write`. The
    // `read_from_written` task has task `write` as origin to prevent hidden dependencies.
    let write = WriteMap::<Key, _>::new("write-read key", Constant);
    let read_from_written = ReadMap::<Key, _>::with_origin("write-read key", write);
    pie.new_session().require(&read_from_written);
  }

  {
    println!("\nTyped key to boxed trait object value:");

    let map = pie.resource_state_mut().get_global_map_mut();
    map.insert(MapKeyToObj::new("manual key 1"), Box::new(1));
    map.insert(MapKeyToObj::new("manual key 2"), Box::new(true));
    let mut session = pie.new_session();
    session.require(&ReadMap::<MapKeyToObj<_>, _>::new("manual key 1"));
    session.require(&ReadMap::<MapKeyToObj<_>, _>::new("manual key 2"));
  }

  {
    println!("\nBoxed trait object key to boxed trait object value:");

    let map = pie.resource_state_mut().get_global_map_mut();
    map.insert(MapKeyObjToObj::from(true), Box::new(1));
    map.insert(MapKeyObjToObj::from(1), Box::new(true));
    let mut session = pie.new_session();
    session.require(&ReadMap::<MapKeyObjToObj, _>::new(Box::new(true)));
    session.require(&ReadMap::<MapKeyObjToObj, _>::new(Box::new(1)));
  }

  Ok(())
}
