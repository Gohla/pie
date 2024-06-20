use std::error::Error;
use std::fmt::Debug;

use dyn_clone::DynClone;

use crate::{OutputChecker, Resource, ResourceChecker, Task};
use crate::context::top_down::TopDownCheck;
use crate::pie::Tracking;
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::collection::TypeToAnyMap;
use crate::trait_object::resource::ResourceCheckerObj;
use crate::trait_object::task::OutputCheckerObj;

/// Internal type for task dependencies.
#[derive(Clone, Debug)]
pub struct TaskDependency<T: Task> {
  task: T,
  checker: Box<dyn OutputCheckerObj<T::Output>>,
  stamp: Box<dyn ValueObj>,
}
impl<T: Task> TaskDependency<T> {
  #[inline]
  pub fn new(task: T, checker: Box<dyn OutputCheckerObj<T::Output>>, stamp: Box<dyn ValueObj>) -> Self {
    Self { task, checker, stamp }
  }
  #[inline]
  pub fn from_typed<C: OutputChecker<T::Output>>(task: T, checker: C, stamp: C::Stamp) -> Self {
    Self::new(task, Box::new(checker), Box::new(stamp))
  }

  #[inline]
  pub fn task(&self) -> &T { &self.task }
  #[inline]
  pub fn checker(&self) -> &dyn OutputCheckerObj<T::Output> { self.checker.as_ref() }
  #[inline]
  pub fn stamp(&self) -> &dyn ValueObj { self.stamp.as_ref() }

  #[inline]
  pub fn check<'i>(&'i self, output: &'i T::Output) -> Option<impl Debug + 'i> {
    self.checker.check_obj(output, self.stamp())
  }

  #[inline]
  pub fn into_require(self) -> Dependency { Dependency::from(self) }
}

/// Internal object-safe trait for task dependencies.
pub trait TaskDependencyObj: DynClone + Debug {
  fn task(&self) -> &dyn KeyObj;
  fn checker(&self) -> &dyn KeyObj;
  fn stamp(&self) -> &dyn ValueObj;

  fn as_top_down_check(&self) -> &dyn TopDownCheck;
  fn is_consistent_bottom_up(&self, output: &dyn ValueObj, requiring_task: &dyn KeyObj, tracker: &mut Tracking) -> bool;
}
const_assert_object_safe!(dyn TaskDependencyObj);
impl<T: Task> TaskDependencyObj for TaskDependency<T> {
  #[inline]
  fn task(&self) -> &dyn KeyObj { &self.task as &dyn KeyObj }
  #[inline]
  fn checker(&self) -> &dyn KeyObj { self.checker.as_ref().as_key_obj() }
  #[inline]
  fn stamp(&self) -> &dyn ValueObj { self.stamp.as_ref() }

  #[inline]
  fn as_top_down_check(&self) -> &dyn TopDownCheck { self as &dyn TopDownCheck }
  #[inline]
  fn is_consistent_bottom_up(&self, output: &dyn ValueObj, requiring_task: &dyn KeyObj, tracker: &mut Tracking) -> bool {
    let Some(output) = output.as_any().downcast_ref::<T::Output>() else {
      return false;
    };
    let check_task_end = tracker.check_task_require_task(requiring_task, self.checker().as_key_obj(), self.stamp());
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
impl<T: Task> From<TaskDependency<T>> for Box<dyn TaskDependencyObj> {
  #[inline]
  fn from(value: TaskDependency<T>) -> Self { Box::new(value) }
}
impl Clone for Box<dyn TaskDependencyObj> {
  #[inline]
  fn clone(&self) -> Self { dyn_clone::clone_box(self.as_ref()) }
}


/// Internal type for resource dependencies.
#[derive(Clone, Debug)]
pub struct ResourceDependency<R> {
  resource: R,
  checker: Box<dyn ResourceCheckerObj<R>>,
  stamp: Box<dyn ValueObj>,
}
impl<R: Resource> ResourceDependency<R> {
  #[inline]
  pub fn new(resource: R, checker: Box<dyn ResourceCheckerObj<R>>, stamp: Box<dyn ValueObj>) -> Self {
    Self { resource, checker, stamp }
  }
  #[inline]
  pub fn from_typed<C: ResourceChecker<R>>(resource: R, checker: C, stamp: C::Stamp) -> Self {
    Self::new(resource, Box::new(checker), Box::new(stamp))
  }

  #[inline]
  pub fn resource(&self) -> &R { &self.resource }
  #[inline]
  pub fn checker(&self) -> &dyn ResourceCheckerObj<R> { self.checker.as_ref() }
  #[inline]
  pub fn stamp(&self) -> &dyn ValueObj { self.stamp.as_ref() }

  #[inline]
  pub fn is_consistent<'i>(
    &'i self,
    state: &'i mut TypeToAnyMap,
    tracker: &mut Tracking,
    track_end: impl FnOnce(&mut Tracking, Result<Option<&dyn Debug>, &dyn Error>),
  ) -> Result<bool, Box<dyn Error>> {
    let inconsistency = self.checker.check_obj(&self.resource, state, self.stamp());
    let inconsistency_dyn = inconsistency.as_ref()
      .map(|o| o.as_ref().map(|i| i as &dyn Debug))
      .map_err(|e| e.as_ref());
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
  fn checker(&self) -> &dyn KeyObj;
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
impl<R: Resource> ResourceDependencyObj for ResourceDependency<R> {
  #[inline]
  fn resource(&self) -> &dyn KeyObj { &self.resource as &dyn KeyObj }
  #[inline]
  fn checker(&self) -> &dyn KeyObj { self.checker.as_ref().as_key_obj() }
  #[inline]
  fn stamp(&self) -> &dyn ValueObj { self.stamp.as_ref() }

  #[inline]
  fn is_consistent_top_down(
    &self,
    resource_state: &mut TypeToAnyMap,
    tracker: &mut Tracking,
  ) -> Result<bool, Box<dyn Error>> {
    let track_end = tracker.check_resource(&self.resource, self.checker().as_key_obj(), self.stamp());
    self.is_consistent(resource_state, tracker, track_end)
  }
  #[inline]
  fn is_consistent_bottom_up(
    &self,
    resource_state: &mut TypeToAnyMap,
    reading_task: &dyn KeyObj,
    tracker: &mut Tracking,
  ) -> Result<bool, Box<dyn Error>> {
    let track_end = tracker.check_task_read_resource(reading_task, self.checker().as_key_obj(), self.stamp());
    self.is_consistent(resource_state, tracker, track_end)
  }
}
impl<R: Resource> From<ResourceDependency<R>> for Box<dyn ResourceDependencyObj> {
  #[inline]
  fn from(value: ResourceDependency<R>) -> Self { Box::new(value) }
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
impl<T: Task> From<TaskDependency<T>> for Dependency {
  #[inline]
  fn from(value: TaskDependency<T>) -> Self { Self::Require(Box::new(value)) }
}
impl From<Box<dyn TaskDependencyObj>> for Dependency {
  #[inline]
  fn from(value: Box<dyn TaskDependencyObj>) -> Self { Self::Require(value) }
}
impl Dependency {
  #[inline]
  pub fn from_read<R: Resource>(resource_dependency: ResourceDependency<R>) -> Self {
    Self::Read(Box::new(resource_dependency))
  }
  #[inline]
  pub fn from_write<R: Resource>(resource_dependency: ResourceDependency<R>) -> Self {
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
