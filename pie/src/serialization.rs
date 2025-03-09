use serde::de::DeserializeOwned;
use serde_flexitos::{MapRegistry, Registry};

use crate::{Resource, Task};
use crate::dependency::{ResourceDependency, ResourceDependencyObj, TaskDependency, TaskDependencyObj};
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::resource::ResourceCheckerObj;
use crate::trait_object::task::TaskErasedObj;

#[macro_export]
macro_rules! create_registry {
  ($trait_object:ident) => {
    create_registry!($trait_object, serde_flexitos::id::Ident<'static>, serde_flexitos::type_to_ident);
  };
  ($trait_object:ident, $ident:ty, $($type_to_ident:ident)::*) => {
    paste::paste! {
      create_registry!($trait_object, $ident, $($type_to_ident)::*, [<$trait_object:snake:upper _DESERIALIZE_REGISTRY>], [<$trait_object:snake:upper _DESERIALIZE_REGISTRY_DISTRIBUTED_SLICE>]);
    }
  };
  ($trait_object:ident, $ident:ty, $($type_to_ident:ident)::*, $registry:ident, $distributed_slice:ident) => {
    #[linkme::distributed_slice]
    pub static $distributed_slice: [fn(&mut serde_flexitos::MapRegistry<dyn $trait_object, $ident>)] = [..];

    static $registry: once_cell::sync::Lazy<serde_flexitos::MapRegistry<dyn $trait_object, $ident>> = once_cell::sync::Lazy::new(|| {
      let mut registry = serde_flexitos::MapRegistry::<dyn $trait_object, $ident>::new(stringify!($trait_object));
      for registry_fn in $distributed_slice {
        registry_fn(&mut registry);
      }
      registry
    });

    impl<'a> serde::Serialize for dyn $trait_object + 'a {
      #[inline]
      fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        const fn __check_erased_serialize_supertrait<T: ?Sized + $trait_object>() {
          serde_flexitos::ser::require_erased_serialize_impl::<T>();
        }
        serde_flexitos::serialize_trait_object(serializer, <Self as serde_flexitos::id::IdObj<$ident>>::id(self), self)
      }
    }

    impl<'a, 'de> serde::Deserialize<'de> for Box<dyn $trait_object + 'a> {
      #[inline]
      fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        use serde_flexitos::Registry;
        $registry.deserialize_trait_object(deserializer)
      }
    }
  };
}

#[macro_export]
macro_rules! impl_id {
  ($generic:ident<$arg:ty>) => {
    impl serde_flexitos::id::Id<$ident> for $generic<$arg> {
      const ID: $ident = serde_flexitos::type_to_ident!($generic<$arg>);
    }
  };
  ($concrete:ty) => {
    impl serde_flexitos::id::Id<$ident> for $concrete {
      const ID: $ident = serde_flexitos::type_to_ident!($concrete);
    }
  };
}

#[macro_export]
macro_rules! impl_into_box {
  ($trait_object:ident, $concrete:ty) => {
    impl Into<Box<dyn $trait_object>> for $concrete {
      #[inline]
      fn into(self) -> Box<dyn $trait_object> {
        Box::new(self)
      }
    }
  }
}

#[macro_export]
macro_rules! register {
  ($trait_object:ident, $concrete:ty) => {
    register!($trait_object, $concrete, [<$trait_object:snake:upper _DESERIALIZE_REGISTRY_DISTRIBUTED_SLICE>]);
  };
  ($trait_object:ident, $concrete:ty, $distributed_slice:ident) => {
    paste::paste! {
      #[linkme::distributed_slice($distributed_slice)]
      #[inline]
      fn [< __register_ $concrete:snake >](registry: &mut serde_flexitos::MapRegistry<dyn $trait_object>) {
        use serde_flexitos::Registry;
        registry.register_id_type::<$concrete>();
      }
    }
  }
}

create_registry!(TaskErasedObj);
create_registry!(TaskDependencyObj);

#[macro_export]
macro_rules! register_task {
  ($task:ty) => {
    impl_id!($task)
    impl_into_box!(TaskErasedObj, $task)
    register!(TaskErasedObj, $task)

    impl_id!(TaskDependency<$task>)
  }
}

pub struct Registries {
  task_registry: MapRegistry<dyn TaskErasedObj>,
  output_registry: MapRegistry<dyn ValueObj>,
  task_dependency_registry: MapRegistry<dyn TaskDependencyObj>,

  resource_registry: MapRegistry<dyn KeyObj>,
  resource_dependency_registry: MapRegistry<dyn ResourceDependencyObj>,
}
impl Registries {
  pub fn new() -> Self {
    Self {
      task_registry: MapRegistry::new("TaskObj"),
      output_registry: MapRegistry::new("ValueObj"),
      task_dependency_registry: MapRegistry::new("TaskDependencyObj"),

      resource_registry: MapRegistry::new("KeyObj"),
      resource_dependency_registry: MapRegistry::new("ResourceDependencyObj"),
    }
  }

  pub fn register_task<T>(&mut self, task_id: &'static str, task_output_id: &'static str) where
    T: Task + DeserializeOwned,
    T::Output: DeserializeOwned,
    TaskDependency<T>: DeserializeOwned,
  {
    self.task_registry.register_type::<T>(task_id);
    // TODO: handle duplicate task output types, which will definitely happen.
    self.output_registry.register(task_output_id, |d| {
      let deserialized = erased_serde::deserialize::<T::Output>(d)?;
      let boxed = Box::new(deserialized);
      Ok(boxed)
    });
    self.task_dependency_registry.register_type::<TaskDependency<T>>(task_id);
  }

  pub fn register_resource<R>(&mut self, resource_id: &'static str) where
    R: Resource + DeserializeOwned,
    ResourceDependency<R>: DeserializeOwned,
  {
    self.resource_registry.register(resource_id, |d| {
      let deserialized = erased_serde::deserialize::<R>(d)?;
      let boxed = Box::new(deserialized);
      Ok(boxed)
    });
    self.resource_dependency_registry.register_type::<ResourceDependency<R>>(resource_id);
  }
}
