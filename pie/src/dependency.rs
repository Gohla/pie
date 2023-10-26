use std::error::Error;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::{OutputChecker, Resource, ResourceChecker, ResourceState, Task};
use crate::context::top_down::CheckTaskDependency;
use crate::pie::Tracking;
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::collection::TypeToAnyMap;

/// Internal type for task dependencies.
#[derive(Clone, Debug)]
pub struct TaskDependency<T, C, S> {
  task: T,
  checker: C,
  stamp: S,
}

impl<T: Task, C: OutputChecker<T::Output>> TaskDependency<T, C, C::Stamp> {
  #[inline]
  pub fn new(task: T, checker: C, stamp: C::Stamp) -> Self { Self { task, checker, stamp } }

  #[inline]
  pub fn task(&self) -> &T { &self.task }
  #[inline]
  pub fn checker(&self) -> &C { &self.checker }
  #[inline]
  pub fn stamp(&self) -> &C::Stamp { &self.stamp }

  #[inline]
  pub fn is_inconsistent_with<'o>(&'o self, output: &'o T::Output) -> Option<C::Inconsistency<'o>> {
    self.checker.is_inconsistent(output, &self.stamp)
  }

  #[inline]
  pub fn into_require(self) -> Dependency { Dependency::from(self) }
}

/// Internal trait for task dependencies.
///
/// Object-safe trait.
pub trait TaskDependencyObj: DynClone + Debug {
  fn task(&self) -> &dyn KeyObj;
  fn checker(&self) -> &dyn ValueObj;
  fn stamp(&self) -> &dyn ValueObj;

  fn as_check_task_dependency(&self) -> &dyn CheckTaskDependency;
  fn is_consistent_with(&self, output: &dyn ValueObj) -> bool;
}

impl<T: Task, C: OutputChecker<T::Output>> TaskDependencyObj for TaskDependency<T, C, C::Stamp> {
  #[inline]
  fn task(&self) -> &dyn KeyObj { &self.task as &dyn KeyObj }
  #[inline]
  fn checker(&self) -> &dyn ValueObj { &self.checker as &dyn ValueObj }
  #[inline]
  fn stamp(&self) -> &dyn ValueObj { &self.stamp as &dyn ValueObj }

  #[inline]
  fn as_check_task_dependency(&self) -> &dyn CheckTaskDependency { self as &dyn CheckTaskDependency }

  #[inline]
  fn is_consistent_with(&self, output: &dyn ValueObj) -> bool {
    // TODO: tracking
    let Some(output) = output.as_any().downcast_ref::<T::Output>() else {
      return false;
    };
    let result = self.is_inconsistent_with(output);
    result.is_none()
  }
}
impl Clone for Box<dyn TaskDependencyObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}


/// Internal type for resource dependencies.
#[derive(Clone, Debug)]
pub struct ResourceDependency<R, C, S> {
  resource: R,
  checker: C,
  stamp: S,
}

impl<R: Resource, C: ResourceChecker<R>> ResourceDependency<R, C, C::Stamp> {
  #[inline]
  pub fn new(resource: R, checker: C, stamp: C::Stamp) -> Self { Self { resource, checker, stamp } }

  #[inline]
  pub fn resource(&self) -> &R { &self.resource }
  #[inline]
  pub fn checker(&self) -> &C { &self.checker }
  #[inline]
  pub fn stamp(&self) -> &C::Stamp { &self.stamp }

  #[inline]
  pub fn is_inconsistent<'i, RS: ResourceState<R>>(
    &'i self,
    state: &'i mut RS,
  ) -> Result<Option<C::Inconsistency<'i>>, C::Error> {
    self.checker.is_inconsistent(&self.resource, state, &self.stamp)
  }

  #[inline]
  pub fn into_read(self) -> Dependency { Dependency::from_read(self) }
  #[inline]
  pub fn into_write(self) -> Dependency { Dependency::from_write(self) }
}

/// Internal trait for resource dependencies.
///
/// Object-safe trait.
pub trait ResourceDependencyObj: DynClone + Debug {
  fn resource(&self) -> &dyn KeyObj;
  fn checker(&self) -> &dyn ValueObj;
  fn stamp(&self) -> &dyn ValueObj;

  fn is_consistent(&self, tracker: &mut Tracking, resource_state: &mut TypeToAnyMap) -> Result<bool, Box<dyn Error>>;
}
impl<R: Resource, C: ResourceChecker<R>> ResourceDependencyObj for ResourceDependency<R, C, C::Stamp> {
  #[inline]
  fn resource(&self) -> &dyn KeyObj { &self.resource as &dyn KeyObj }
  #[inline]
  fn checker(&self) -> &dyn ValueObj { &self.checker as &dyn ValueObj }
  #[inline]
  fn stamp(&self) -> &dyn ValueObj { &self.stamp as &dyn ValueObj }

  #[inline]
  fn is_consistent(&self, tracker: &mut Tracking, state: &mut TypeToAnyMap) -> Result<bool, Box<dyn Error>> {
    let check_resource_end = tracker.check_resource(&self.resource, &self.checker, &self.stamp);
    let result = self.is_inconsistent(state);
    let inconsistency = result.as_ref()
      .map(|o| o.as_ref().map(|i| i as &dyn Debug))
      .map_err(|e| e as &dyn Error);
    check_resource_end(tracker, inconsistency);
    Ok(result?.is_none())
  }
}
impl Clone for Box<dyn ResourceDependencyObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}


/// Enumeration of all kinds of dependencies.
#[derive(Clone, Debug)]
pub enum Dependency {
  ReservedRequire,
  Require(Box<dyn TaskDependencyObj>),
  Read(Box<dyn ResourceDependencyObj>),
  Write(Box<dyn ResourceDependencyObj>),
}
impl<T: Task, C: OutputChecker<T::Output>> From<TaskDependency<T, C, C::Stamp>> for Dependency {
  #[inline]
  fn from(value: TaskDependency<T, C, C::Stamp>) -> Self { Self::Require(Box::new(value)) }
}
impl Dependency {
  #[inline]
  pub fn from_read<R: Resource, C: ResourceChecker<R>>(resource_dependency: ResourceDependency<R, C, C::Stamp>) -> Self {
    Self::Read(Box::new(resource_dependency))
  }
  #[inline]
  pub fn from_write<R: Resource, C: ResourceChecker<R>>(resource_dependency: ResourceDependency<R, C, C::Stamp>) -> Self {
    Self::Write(Box::new(resource_dependency))
  }
}

// Note: this PartialEq implementation only checks the tasks and resources of dependencies.
impl PartialEq for Dependency {
  fn eq(&self, other: &Self) -> bool {
    match (self, other) {
      (Self::Require(d), Self::Require(o)) => d.task() == o.task(),
      (Self::Read(d), Self::Read(o)) => d.resource() == o.resource(),
      (Self::Write(d), Self::Write(o)) => d.resource() == o.resource(),
      (Self::ReservedRequire, Self::ReservedRequire) => true,
      _ => false,
    }
  }
}
