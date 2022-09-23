use std::collections::HashMap;

use linkme::distributed_slice;
use serde::de::DeserializeOwned;
use crate::DynTask;

pub type DeserializeFn<T> = fn(&mut dyn erased_serde::Deserializer) -> erased_serde::Result<Box<T>>;

pub struct Registry<T: ?Sized> {
  map: HashMap<String, DeserializeFn<T>>,
}

impl<T> Registry<T> {
  pub fn register(&mut self, name: String, deserialize_fn: DeserializeFn<T>) {
    self.map.insert(name, deserialize_fn);
  }
}

/// Distributed slice for registering deserialization functions
#[distributed_slice]
pub static TASK_DESERIALIZE_FNS: [fn(&mut Registry<dyn DynTask>)] = [..];
