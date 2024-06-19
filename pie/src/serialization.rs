use serde::de::DeserializeOwned;
use serde_flexitos::{MapRegistry, Registry};

use crate::{Resource, Task};
use crate::dependency::{ResourceDependency, ResourceDependencyObj, TaskDependency, TaskDependencyObj};
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::task::TaskObj;

pub struct Registries {
  task_registry: MapRegistry<dyn TaskObj>,
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
