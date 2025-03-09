pub use inner::{MaybeErasedSerialize, MaybeId, MaybeIdObj, MaybeSerialize};

#[cfg(feature = "serde")]
mod inner {
  pub trait MaybeSerialize: serde::Serialize {}
  impl<T: serde::Serialize> MaybeSerialize for T {}

  pub trait MaybeId: serde_flexitos::id::Id {}
  impl<T: serde_flexitos::id::Id> MaybeId for T {}

  pub trait MaybeErasedSerialize: erased_serde::Serialize {}
  impl<T: erased_serde::Serialize + ?Sized> MaybeErasedSerialize for T {}

  pub trait MaybeIdObj: serde_flexitos::id::IdObj {}
  impl<T: serde_flexitos::id::IdObj + ?Sized> MaybeIdObj for T {}
}

#[cfg(not(feature = "serde"))]
mod inner {
  pub trait MaybeSerialize {}
  impl<T> MaybeSerialize for T {}

  pub trait MaybeId {}
  impl<T> MaybeId for T {}

  pub trait MaybeErasedSerialize {}
  impl<T> crate::serialize::MaybeErasedSerialize for T {}

  pub trait MaybeIdObj {}
  impl<T> MaybeIdObj for T {}
}
