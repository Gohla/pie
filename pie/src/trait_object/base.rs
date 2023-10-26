use std::any::Any;
use std::hash::{Hash, Hasher};

/// Conversion into [`&dyn Any`](Any). Implies `'static` because [`Any`] requires `'static`.
pub trait AsAny: 'static {
  fn as_any(&self) -> &dyn Any;
  fn as_box_any(self: Box<Self>) -> Box<dyn Any>;
}
impl<T: Any> AsAny for T {
  #[inline]
  fn as_any(&self) -> &dyn Any { self as &dyn Any }
  #[inline]
  fn as_box_any(self: Box<Self>) -> Box<dyn Any> { self as Box<dyn Any> }
}

/// Object safe proxy of [`Eq`], comparing against [`&dyn Any`](Any).
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

/// Object safe proxy of [`Hash`].
pub trait HashObj {
  fn hash_obj(&self, state: &mut dyn Hasher);
}
impl<T: Hash> HashObj for T {
  #[inline]
  fn hash_obj(&self, mut state: &mut dyn Hasher) { self.hash(&mut state); }
}

/// Clone `&self` into `Box<O>` where `O` can be a trait object.
pub trait CloneBox<O: ?Sized> {
  fn clone_box(&self) -> Box<O>;
}
