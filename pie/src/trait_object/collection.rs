use std::any::{Any, TypeId};
use std::collections::HashMap;

use crate::{Resource, ResourceState};

/// Hash map from types (through [`TypeId`]) to any value (through [`Box<dyn Any>`](Any)).
#[derive(Default)]
#[repr(transparent)]
pub struct TypeToAnyMap(HashMap<TypeId, Box<dyn Any>>);
impl TypeToAnyMap {
  #[inline]
  /// Gets whether a value for [`T`] exists.
  fn exists<T: ?Sized + Any>(&self) -> bool {
    self.0.contains_key(&TypeId::of::<T>())
  }
  #[inline]
  /// Gets whether a value for [`T`] exists, and if it is of type [`V`].
  fn is_of_type<T: ?Sized + Any, V: Any>(&self) -> bool {
    self.0.get(&TypeId::of::<T>()).map(|v| v.is::<V>()).unwrap_or_default()
  }

  /// Gets a reference to the concrete value (of type [`V`]) for type [`T`], returning `Some(&value)` if a value for
  /// [`T`] exists and is of type [`V`], or `None` otherwise.
  #[inline]
  pub fn get<T: ?Sized + Any, V: Any>(&self) -> Option<&V> {
    self.get_any::<T>().and_then(|v| v.downcast_ref::<V>())
  }
  /// Gets a mutable reference to the value (of type [`V`]) for type [`T`], returning `Some(&mut value)` if a value for
  /// [`T`] exists and is of type [`V`], or `None` otherwise.
  #[inline]
  pub fn get_mut<T: ?Sized + Any, V: Any>(&mut self) -> Option<&mut V> {
    self.get_any_mut::<T>().and_then(|v| v.downcast_mut::<V>())
  }
  /// Sets the `value` (of type [`V`]) for type [`T`].
  #[inline]
  pub fn set<T: ?Sized + Any, V: Any>(&mut self, value: V) {
    let type_id = TypeId::of::<T>();
    if let Some(any) = self.0.get_mut(&type_id) {
      if let Some(existing_value) = any.downcast_mut::<V>() {
        *existing_value = value;
      } else {
        *any = Box::new(value)
      }
    } else {
      self.0.insert(type_id, Box::new(value));
    }
  }

  /// Gets a reference to the [`dyn Any`] value for type [`T`], returning `Some(&value)` if a value for [`T`] exists, or
  /// `None` otherwise.
  #[inline]
  pub fn get_any<T: ?Sized + Any>(&self) -> Option<&dyn Any> {
    self.get_boxed_any::<T>().map(|any| any.as_ref())
  }
  /// Gets a mutable reference to the [`dyn Any`] value for type [`T`], returning `Some(&mut value)` if a value for
  /// [`T`] exists, or `None` otherwise.
  #[inline]
  pub fn get_any_mut<T: ?Sized + Any>(&mut self) -> Option<&mut dyn Any> {
    self.get_boxed_any_mut::<T>().map(|any| any.as_mut())
  }

  /// Gets a reference to the [`Box<dyn Any>`] value for type [`T`], returning `Some(&value)` if a value for [`T`]
  /// exists, or `None` otherwise.
  #[inline]
  pub fn get_boxed_any<T: ?Sized + Any>(&self) -> Option<&Box<dyn Any>> {
    self.0.get(&TypeId::of::<T>())
  }
  /// Gets a mutable reference to the [`Box<dyn Any>`] value for type [`T`], returning `Some(&mut value)` if a value for
  /// [`T`] exists, or `None` otherwise.
  #[inline]
  pub fn get_boxed_any_mut<T: ?Sized + Any>(&mut self) -> Option<&mut Box<dyn Any>> {
    self.0.get_mut(&TypeId::of::<T>())
  }
  /// Sets the [`Box<dyn Any>`] `value` for type [`T`].
  #[inline]
  pub fn set_boxed_any<T: ?Sized + Any>(&mut self, value: Box<dyn Any>) {
    self.0.insert(TypeId::of::<T>(), value);
  }

  /// Gets a reference to the value of type `V` for type `T`.
  ///
  /// If no value has been set, the value is first set to `V::default()`. If a value of a different type (not `V`) has
  /// been set, the value is first replaced to `V::default()`.
  #[inline]
  pub fn get_or_set_default<T: ?Sized + Any, V: Default + Any>(&mut self) -> &V {
    self.ensure_inserted_and_correct_type::<T, V>().downcast_ref::<V>().unwrap()
  }
  /// Gets a mutable reference to the value of type `V` for type `T`.
  ///
  /// If no value has been set, the value is first set to `V::default()`. If a value of a different type (not `V`) has
  /// been set, the value is first replaced to `V::default()`.
  #[inline]
  pub fn get_or_set_default_mut<T: ?Sized + Any, V: Default + Any>(&mut self) -> &mut V {
    self.ensure_inserted_and_correct_type::<T, V>().downcast_mut::<V>().unwrap()
  }

  #[inline]
  fn ensure_inserted_and_correct_type<T: ?Sized + Any, V: Any + Default>(&mut self) -> &mut dyn Any {
    let box_any = self.0.entry(TypeId::of::<T>())
      .and_modify(|value|
        if !value.as_ref().is::<V>() {
          *value = Box::new(V::default());
        }
      )
      .or_insert_with(|| Box::new(V::default()));
    // NOTE: explicitly convert `&mut Box<dyn Any>` to `&mut dyn Any` with `as_mut`, to get to the actual value in the
    //       box. Otherwise, implicit conversion will convert the Box to `&mut dyn Any`, but that will cause subsequent
    //       downcast methods to fail, because they will try to downcast the box, not the value in the box!
    let mut_any = box_any.as_mut();
    mut_any
  }
}

impl<R: Resource> ResourceState<R> for TypeToAnyMap {
  #[inline]
  fn exists(&self) -> bool { self.exists::<R>() }
  #[inline]
  fn is_of_type<S: Any>(&self) -> bool { self.is_of_type::<R, S>() }

  #[inline]
  fn get<S: Any>(&self) -> Option<&S> { self.get::<R, S>() }
  #[inline]
  fn get_mut<S: Any>(&mut self) -> Option<&mut S> { self.get_mut::<R, S>() }
  #[inline]
  fn set<S: Any>(&mut self, state: S) { self.set::<R, S>(state) }

  #[inline]
  fn get_any(&self) -> Option<&dyn Any> { self.get_any::<R>() }
  #[inline]
  fn get_any_mut(&mut self) -> Option<&mut dyn Any> { self.get_any_mut::<R>() }

  #[inline]
  fn get_boxed_any(&self) -> Option<&Box<dyn Any>> { self.get_boxed_any::<R>() }
  #[inline]
  fn get_boxed_any_mut(&mut self) -> Option<&mut Box<dyn Any>> { self.get_boxed_any_mut::<R>() }
  #[inline]
  fn set_boxed_any(&mut self, state: Box<dyn Any>) { self.set_boxed_any::<R>(state) }

  #[inline]
  fn get_or_set_default<S: Default + Any>(&mut self) -> &S { self.get_or_set_default::<R, S>() }
  #[inline]
  fn get_or_set_default_mut<S: Default + Any>(&mut self) -> &mut S { self.get_or_set_default_mut::<R, S>() }
}
