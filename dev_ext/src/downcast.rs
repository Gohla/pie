use std::any::Any;

use dev_util::downcast_ref_or_panic;
use pie::trait_object::{KeyObj, ValueObj};

/// Downcast trait objects to specific types for testing purposes.
pub trait Downcast {
  fn downcast<T: Any>(&self) -> &T;

  fn as_str(&self) -> &'static str {
    *self.downcast::<&'static str>()
  }
  fn as_result_str<E: Any>(&self) -> &Result<&'static str, E> {
    self.downcast::<Result<&'static str, E>>()
  }

  fn as_string(&self) -> &String {
    self.downcast::<String>()
  }
  fn as_result_string<E: Any>(&self) -> &Result<String, E> {
    self.downcast::<Result<String, E>>()
  }
}

impl Downcast for dyn ValueObj {
  fn downcast<T: Any>(&self) -> &T { downcast_ref_or_panic::<T>(self.as_any()) }
}
impl Downcast for Box<dyn ValueObj> {
  fn downcast<T: Any>(&self) -> &T { downcast_ref_or_panic::<T>(self.as_ref().as_any()) }
}

impl Downcast for dyn KeyObj {
  fn downcast<T: Any>(&self) -> &T { downcast_ref_or_panic::<T>(self.as_any()) }
}
impl Downcast for Box<dyn KeyObj> {
  fn downcast<T: Any>(&self) -> &T { downcast_ref_or_panic::<T>(self.as_ref().as_any()) }
}
