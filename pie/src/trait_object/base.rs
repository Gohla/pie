use std::any::Any;
use std::hash::{Hash, Hasher};

/// Conversion into [`dyn Any`](Any). Implies `'static` because [`Any`] requires `'static`.
pub trait AsAny: 'static {
  /// Convert `&self` into [`&dyn Any`](Any).
  fn as_any(&self) -> &dyn Any;
  /// Convert `Box<Self>`  into [`Box<dyn Any>`](Any).
  fn into_box_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T: Any> AsAny for T {
  #[inline]
  fn as_any(&self) -> &dyn Any { self as &dyn Any }
  #[inline]
  fn into_box_any(self: Box<Self>) -> Box<dyn Any> { self as Box<dyn Any> }
}


/// Object safe [`Eq`] proxy, comparing against [`&dyn Any`](Any).
pub trait EqObj {
  fn eq_any(&self, other: &dyn Any) -> bool;
}

impl<T: Eq + Any> EqObj for T {
  #[inline]
  fn eq_any(&self, other: &dyn Any) -> bool {
    if let Some(other) = other.downcast_ref::<Self>() {
      self == other
    } else {
      false
    }
  }
}


/// Object safe [`Hash`] proxy.
pub trait HashObj {
  fn hash_obj(&self, state: &mut dyn Hasher);
}

impl<T: Hash> HashObj for T {
  #[inline]
  fn hash_obj(&self, mut state: &mut dyn Hasher) { self.hash(&mut state); }
}


/// Assert that given type is object-safe at compile-time.
macro_rules! const_assert_object_safe {
    ($ty:ty) => {
        const _: () = { let _: &$ty; assert!(true) };
    }
}
