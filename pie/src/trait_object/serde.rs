use pie_tagged_serde::{impl_registry, Registry};

// Tasks

impl_registry!(dyn crate::trait_object::DynTask, TASK_REGISTRY_FNS, TASK_REGISTRY);

/// Implements [`Id`] for `$concrete`, `From<$concrete>` for `Box<dyn DynTask>`, and registers a registration function 
/// for `$concrete` with the distributed slice at `TASK_REGISTRY_FNS`.
#[macro_export]
macro_rules! register_task {
  ($concrete:ty) => {
    pie_tagged_serde::register!($concrete, dyn $crate::trait_object::DynTask, $crate::trait_object::serde::TASK_REGISTRY_FNS);
  }
}


// Dependencies

impl_registry!(dyn crate::trait_object::DynDependency, DEPENDENCY_REGISTRY_FNS, DEPENDENCY_REGISTRY);

/// Implements [`Id`] for `$concrete`, `From<$concrete>` for `Box<dyn DynTask>`, and registers a registration function 
/// for `$concrete` with the distributed slice at `DEPENDENCY_REGISTRY_FNS`.
#[macro_export]
macro_rules! register_dependency {
  ($concrete:ty) => {
    pie_tagged_serde::register!($concrete, dyn $crate::trait_object::DynDependency, $crate::trait_object::serde::DEPENDENCY_REGISTRY_FNS);
  }
}
