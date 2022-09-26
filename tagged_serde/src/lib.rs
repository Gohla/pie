use std::collections::BTreeMap;
use std::fmt;

pub use linkme::distributed_slice;
pub use paste::paste;
pub use lazy_static::lazy_static;

use serde::{de, Deserialize, Deserializer, Serialize, Serializer};
use serde::de::{DeserializeSeed, Expected, MapAccess, Visitor};
use serde::ser::SerializeMap;

/// Trait for getting the unique identifier of a type, for the purposes of tagged (de)serialization.
pub trait Id {
  /// Gets the unique identifier of this type.
  fn id() -> &'static str;
}

/// Trait-object-safe version of [`Id`].
pub trait DynId {
  /// Gets the unique identifier of this type. This is a method instead of a function because this
  /// trait must be object-safe; traits with associated functions are not object-safe.
  fn dyn_id(&self) -> &'static str;
}

impl<T: Id + ?Sized> DynId for T {
  #[inline]
  fn dyn_id(&self) -> &'static str { T::id() }
}

/// Registry for mapping unique identifiers from [`Id`] to a function that deserializes to instances 
/// of the type of the identifier.
pub struct Registry<O: ?Sized> {
  map: BTreeMap<&'static str, Option<DeserializeFn<O>>>,
  names: Vec<&'static str>,
}

impl<O: ?Sized> Default for Registry<O> {
  #[inline]
  fn default() -> Self { Self { map: BTreeMap::new(), names: Vec::new() } }
}

/// Type alias for the deserialization function.
pub type DeserializeFn<O> = for<'de> fn(&mut dyn erased_serde::Deserializer<'de>) -> erased_serde::Result<Box<O>>;

impl<O: ?Sized> Registry<O> {
  /// Creates a new empty registry.
  pub fn new() -> Self { Self::default() }

  /// Registers given type with the registry.
  pub fn register<T: Id + for<'de> serde::Deserialize<'de> + Into<Box<O>> + 'static>(&mut self) {
    let id = T::id();
    self.map.insert(id, Some(deserialize_fn::<T, O>));
    self.names.push(id);
  }
}

/// Trait for providing a [`Registry`] instance for a specific trait-object, along with the name of the trait object for
/// error reporting.
pub trait RegistryProvider {
  fn registry() -> &'static Registry<Self>;
  fn trait_object_name() -> &'static str;
}

/// Wrapper for tagged serialization where a type is serialized along with its identifier, enabling deserialization of 
/// a trait object.
#[repr(transparent)]
#[derive(Debug)]
pub struct TaggedSerde<O: ?Sized>(pub Box<O>);

impl<O: ?Sized> TaggedSerde<O> {
  pub fn new(value: Box<O>) -> Self { Self(value) }
}

/// Defines a distributed slice for registration functions with id `$distributed_slice_id`, defines a static registry
/// with name `$registry_id` of type `Registry<$trait_object>` that applies all registration functions, and implements
/// [`RegistryProvider`] for `$trait_object`.
#[macro_export]
macro_rules! impl_registry {
  ($trait_object:ty, $distributed_slice_id:ident, $registry_id:ident) => {
    #[$crate::distributed_slice]
    pub static $distributed_slice_id: [fn(&mut $crate::Registry<$trait_object>)] = [..];
    
    $crate::lazy_static! {
      static ref $registry_id: Registry<$trait_object> = {
        let mut registry = $crate::Registry::new();
        for registry_fn in $distributed_slice_id {
          registry_fn(&mut registry);
        }
        registry
      };
    }
    
    impl $crate::RegistryProvider for $trait_object {
      #[inline]
      fn registry() -> &'static Registry<Self> { &$registry_id }
      #[inline]
      fn trait_object_name() -> &'static str { stringify!($trait_object) }
    }
  }
}

/// Implements [`Id`] for `$concrete`, `From<$concrete>` for `Box<$trait_object>`, and registers
/// a registration function for `$concrete` with the distributed slice at `$distributed_slice_path`.
#[macro_export]
macro_rules! register {
  ($concrete:ty, $trait_object:ty, $distributed_slice_path:path) => {
    impl $crate::Id for $concrete {
      #[inline]
      fn id() -> &'static str { stringify!($concrete) }
    }
    
    impl From<$concrete> for Box<$trait_object> {
      #[inline]
      fn from(v: $concrete) -> Self { Box::new(v) }
    }
    
    $crate::paste! {
      #[$crate::distributed_slice($distributed_slice_path)]
      fn [< __register_ $concrete:snake >](registry: &mut $crate::Registry<$trait_object>) {
        registry.register::<$concrete>();
      }
    }
  }
}


// Internals

fn deserialize_fn<T: for<'de> serde::Deserialize<'de> + Into<Box<O>> + 'static, O: ?Sized>(deserializer: &mut dyn erased_serde::Deserializer) -> erased_serde::Result<Box<O>> {
  Ok(erased_serde::deserialize::<T>(deserializer)?.into())
}

impl<O: DynId + Serialize + ?Sized> Serialize for TaggedSerde<O> {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    let mut serializer = serializer.serialize_map(Some(1))?;
    serializer.serialize_entry(self.0.dyn_id(), &self.0)?;
    serializer.end()
  }
}

impl<'de, O: RegistryProvider + ?Sized + 'static> Deserialize<'de> for TaggedSerde<O> {
  fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
    let visitor = TaggedVisitor {
      trait_object: O::trait_object_name(),
      registry: O::registry(),
    };
    deserializer.deserialize_map(visitor).map(|v| TaggedSerde::new(v))
  }
}

struct TaggedVisitor<T: ?Sized + 'static> {
  trait_object: &'static str,
  registry: &'static Registry<T>,
}

impl<'de, T: ?Sized> Visitor<'de> for TaggedVisitor<T> {
  type Value = Box<T>;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    write!(formatter, "dyn {}", self.trait_object)
  }

  fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Self::Value, A::Error> {
    let map_lookup = MapLookupVisitor {
      expected: &self,
      registry: self.registry,
    };
    let deserialize_fn = match map.next_key_seed(map_lookup)? {
      Some(deserialize_fn) => deserialize_fn,
      None => {
        return Err(de::Error::custom(format_args!(
          "expected externally tagged {}",
          self.trait_object
        )));
      }
    };
    map.next_value_seed(FnApply { deserialize_fn })
  }
}

struct MapLookupVisitor<'a, T: ?Sized + 'static> {
  pub expected: &'a dyn Expected,
  pub registry: &'static Registry<T>,
}

impl<'de, 'a, T: ?Sized + 'static> Visitor<'de> for MapLookupVisitor<'a, T> {
  type Value = DeserializeFn<T>;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    Expected::fmt(self.expected, formatter)
  }

  fn visit_str<E: de::Error>(self, key: &str) -> Result<Self::Value, E> {
    match self.registry.map.get(key) {
      Some(Some(value)) => Ok(*value),
      Some(None) => Err(de::Error::custom(format_args!(
        "non-unique tag of {}: {:?}",
        self.expected, key
      ))),
      None => Err(de::Error::unknown_variant(key, &self.registry.names)),
    }
  }
}

impl<'de, 'a, T: ?Sized + 'static> DeserializeSeed<'de> for MapLookupVisitor<'a, T> {
  type Value = DeserializeFn<T>;

  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    deserializer.deserialize_str(self)
  }
}

pub struct FnApply<T: ?Sized> {
  pub deserialize_fn: DeserializeFn<T>,
}

impl<'de, T: ?Sized> DeserializeSeed<'de> for FnApply<T> {
  type Value = Box<T>;

  fn deserialize<D: Deserializer<'de>>(self, deserializer: D) -> Result<Self::Value, D::Error> {
    let mut erased = <dyn erased_serde::Deserializer>::erase(deserializer);
    (self.deserialize_fn)(&mut erased).map_err(de::Error::custom)
  }
}
