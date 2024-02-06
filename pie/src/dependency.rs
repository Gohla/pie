use std::error::Error;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::{OutputChecker, Resource, ResourceChecker, ResourceState, Task};
use crate::context::top_down::TopDownCheck;
use crate::pie::Tracking;
use crate::trait_object::{KeyObj, ValueEqObj, ValueObj};
use crate::trait_object::collection::TypeToAnyMap;

/// Internal type for task dependencies.
#[derive(Clone, Debug)]
pub struct TaskDependency<T, C, S> {
  task: T,
  checker: C,
  stamp: S,
}
impl<T: Task, C: OutputChecker> TaskDependency<T, C, Box<dyn ValueEqObj>> {
  #[inline]
  pub fn new(task: T, checker: C, stamp: Box<dyn ValueEqObj>) -> Self { Self { task, checker, stamp } }

  #[inline]
  pub fn task(&self) -> &T { &self.task }
  #[inline]
  pub fn checker(&self) -> &C { &self.checker }
  #[inline]
  pub fn stamp(&self) -> &Box<dyn ValueEqObj> { &self.stamp }

  #[inline]
  pub fn check<'i>(&'i self, output: &'i T::Output) -> Option<impl Debug + 'i> {
    self.checker.check(output, self.stamp.as_ref())
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

  fn as_top_down_check(&self) -> &dyn TopDownCheck;
  fn is_consistent_bottom_up(&self, output: &dyn ValueObj, requiring_task: &dyn KeyObj, tracker: &mut Tracking) -> bool;
}
const_assert_object_safe!(dyn TaskDependencyObj);
impl<T: Task, C: OutputChecker> TaskDependencyObj for TaskDependency<T, C, Box<dyn ValueEqObj>> {
  #[inline]
  fn task(&self) -> &dyn KeyObj { &self.task as &dyn KeyObj }
  #[inline]
  fn checker(&self) -> &dyn ValueObj { &self.checker as &dyn ValueObj }
  #[inline]
  fn stamp(&self) -> &dyn ValueObj { &self.stamp as &dyn ValueObj }

  #[inline]
  fn as_top_down_check(&self) -> &dyn TopDownCheck { self as &dyn TopDownCheck }
  #[inline]
  fn is_consistent_bottom_up(&self, output: &dyn ValueObj, requiring_task: &dyn KeyObj, tracker: &mut Tracking) -> bool {
    let Some(output) = output.as_any().downcast_ref::<T::Output>() else {
      return false;
    };
    let check_task_end = tracker.check_task_require_task(requiring_task, &self.checker, &self.stamp);
    match self.check(output) {
      Some(inconsistency) => {
        check_task_end(tracker, Some(&inconsistency as &dyn Debug));
        false
      }
      None => {
        check_task_end(tracker, None);
        true
      }
    }
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
  pub fn check<'i, RS: ResourceState<R>>(
    &'i self,
    state: &'i mut RS,
  ) -> Result<Option<impl Debug + 'i>, C::Error> {
    self.checker.check(&self.resource, state, &self.stamp)
  }

  #[inline]
  pub fn is_consistent<'i, RS: ResourceState<R>>(
    &'i self,
    state: &'i mut RS,
    tracker: &mut Tracking,
    track_end: impl FnOnce(&mut Tracking, Result<Option<&dyn Debug>, &dyn Error>),
  ) -> Result<bool, Box<dyn Error>> {
    let inconsistency = self.check(state);
    let inconsistency_dyn = inconsistency.as_ref()
      .map(|o| o.as_ref().map(|i| i as &dyn Debug))
      .map_err(|e| e as &dyn Error);
    track_end(tracker, inconsistency_dyn);
    Ok(inconsistency?.is_none())
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

  fn is_consistent_top_down(
    &self,
    resource_state: &mut TypeToAnyMap,
    tracker: &mut Tracking,
  ) -> Result<bool, Box<dyn Error>>;
  fn is_consistent_bottom_up(
    &self,
    resource_state: &mut TypeToAnyMap,
    reading_task: &dyn KeyObj,
    tracker: &mut Tracking,
  ) -> Result<bool, Box<dyn Error>>;
}
const_assert_object_safe!(dyn ResourceDependencyObj);
impl<R: Resource, C: ResourceChecker<R>> ResourceDependencyObj for ResourceDependency<R, C, C::Stamp> {
  #[inline]
  fn resource(&self) -> &dyn KeyObj { &self.resource as &dyn KeyObj }
  #[inline]
  fn checker(&self) -> &dyn ValueObj { &self.checker as &dyn ValueObj }
  #[inline]
  fn stamp(&self) -> &dyn ValueObj { &self.stamp as &dyn ValueObj }

  #[inline]
  fn is_consistent_top_down(
    &self,
    resource_state: &mut TypeToAnyMap,
    tracker: &mut Tracking,
  ) -> Result<bool, Box<dyn Error>> {
    let track_end = tracker.check_resource(&self.resource, &self.checker, &self.stamp);
    self.is_consistent(resource_state, tracker, track_end)
  }
  #[inline]
  fn is_consistent_bottom_up(
    &self,
    resource_state: &mut TypeToAnyMap,
    reading_task: &dyn KeyObj,
    tracker: &mut Tracking,
  ) -> Result<bool, Box<dyn Error>> {
    let track_end = tracker.check_task_read_resource(reading_task, &self.checker, &self.stamp);
    self.is_consistent(resource_state, tracker, track_end)
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
impl<T: Task, C: OutputChecker> From<TaskDependency<T, C, Box<dyn ValueEqObj>>> for Dependency {
  #[inline]
  fn from(value: TaskDependency<T, C, Box<dyn ValueEqObj>>) -> Self { Self::Require(Box::new(value)) }
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
