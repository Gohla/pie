use std::collections::HashSet;
use std::error::Error;
use std::fmt::Debug;
use std::ops::{Deref, DerefMut};

use crate::{Context, OutputChecker, Resource, ResourceChecker, ResourceState, Session, Task, Value, ValueEq};
use crate::context::bottom_up::BottomUpContext;
use crate::context::top_down::TopDownContext;
use crate::store::{Store, TaskNode};
use crate::task::AlwaysConsistent;
use crate::tracker::Tracker;
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::collection::TypeToAnyMap;

/// Internals for [Pie](crate::Pie).
pub struct PieInternal<A> {
  store: Store,
  tracker: A,
  resource_state: TypeToAnyMap,
}
impl Default for PieInternal<()> {
  #[inline]
  fn default() -> Self { PieInternal::with_tracker(()) }
}
impl<A: Tracker> PieInternal<A> {
  #[inline]
  pub fn with_tracker(tracker: A) -> Self {
    Self {
      store: Store::default(),
      tracker,
      resource_state: TypeToAnyMap::default(),
    }
  }

  #[inline]
  pub fn new_session(&mut self) -> Session { Session(SessionInternal::new(self)) }
  #[inline]
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session) -> R) -> R { f(self.new_session()) }

  #[inline]
  pub fn tracker(&self) -> &A { &self.tracker }
  #[inline]
  pub fn tracker_mut(&mut self) -> &mut A { &mut self.tracker }

  #[inline]
  pub fn resource_state<R: Resource>(&self) -> &impl ResourceState<R> { &self.resource_state }
  #[inline]
  pub fn resource_state_mut<R: Resource>(&mut self) -> &mut impl ResourceState<R> { &mut self.resource_state }
}

/// Internals for [`Session`].
pub struct SessionInternal<'p> {
  pub store: &'p mut Store,
  pub resource_state: &'p mut TypeToAnyMap,
  pub tracker: Tracking<'p>,
  pub current_executing_task: Option<TaskNode>,
  pub consistent: HashSet<TaskNode>,
  pub dependency_check_errors: Vec<Box<dyn Error>>,
}
impl<'p> SessionInternal<'p> {
  #[inline]
  pub fn new<A: Tracker>(pie: &'p mut PieInternal<A>) -> Self {
    Self {
      store: &mut pie.store,
      resource_state: &mut pie.resource_state,
      tracker: Tracking(&mut pie.tracker as &mut dyn Tracker),
      current_executing_task: None,
      consistent: HashSet::default(),
      dependency_check_errors: Vec::default(),
    }
  }

  #[inline]
  pub fn require<T: Task>(&mut self, task: &T) -> T::Output {
    self.current_executing_task = None;

    let build_end = self.tracker.build();
    let mut context = TopDownContext::new(self);
    let output = context.require(task, AlwaysConsistent);
    build_end(&mut self.tracker);
    output
  }

  #[inline]
  pub fn create_bottom_up_build<'s>(&'s mut self) -> BottomUpBuildInternal<'p, 's> {
    BottomUpBuildInternal(BottomUpContext::new(self))
  }

  #[inline]
  pub fn dependency_check_errors(&self) -> impl Iterator<Item=&dyn Error> + ExactSizeIterator {
    self.dependency_check_errors.iter().map(|e| e.as_ref())
  }
}

/// Internals for [`BottomUpBuildInternal`].
#[repr(transparent)]
pub struct BottomUpBuildInternal<'p, 's>(BottomUpContext<'p, 's>);
impl<'p, 's> BottomUpBuildInternal<'p, 's> {
  #[inline]
  pub fn schedule_tasks_affected_by(&mut self, resource: &dyn KeyObj) {
    self.0.schedule_tasks_affected_by(resource);
  }
  #[inline]
  pub fn update_affected_tasks(mut self) {
    self.0.session.current_executing_task = None;

    let build_end = self.0.session.tracker.build();
    self.0.execute_scheduled();
    build_end(&mut self.0.session.tracker);
  }
}

/// Internal convenience methods for tracking start/end pairs.
#[repr(transparent)]
pub struct Tracking<'p>(pub &'p mut dyn Tracker);
impl Tracking<'_> {
  #[inline]
  #[must_use]
  pub fn build(&mut self) -> impl FnOnce(&mut Tracking) {
    self.0.build_start();
    |tracking| tracking.0.build_end()
  }

  #[inline]
  #[must_use]
  pub fn require<'a, T: Task, C: OutputChecker, S: Value>(
    &mut self,
    task: &'a T,
    checker: &'a C,
  ) -> impl FnOnce(&mut Tracking, &S, &T::Output) + 'a {
    self.0.require_start(task, checker);
    |tracking, stamp, output|
      tracking.0.require_end(task, checker, stamp, output)
  }
  #[inline]
  #[must_use]
  pub fn read<'a, R: Resource, C: ResourceChecker<R>>(
    &mut self,
    resource: &'a R,
    checker: &'a C,
  ) -> impl FnOnce(&mut Tracking, &C::Stamp) + 'a {
    self.0.read_start(resource, checker);
    |tracking, stamp| tracking.0.read_end(resource, checker, stamp)
  }
  #[inline]
  #[must_use]
  pub fn write<'a, R: Resource, C: ResourceChecker<R>>(
    &mut self,
    resource: &'a R,
    checker: &'a C,
  ) -> impl FnOnce(&mut Tracking, &C::Stamp) + 'a {
    self.0.write_start(resource, checker);
    |tracking, stamp| tracking.0.write_end(resource, checker, stamp)
  }

  #[inline]
  #[must_use]
  pub fn check_task<'a, T: Task, C: OutputChecker, S: ValueEq>(
    &mut self,
    task: &'a T,
    checker: &'a C,
    stamp: &'a S,
  ) -> impl FnOnce(&mut Tracking, Option<&dyn Debug>) + 'a {
    self.0.check_task_start(task, checker, stamp);
    |tracking, inconsistency| tracking.0.check_task_end(task, checker, stamp, inconsistency)
  }
  #[inline]
  #[must_use]
  pub fn check_resource<'a, R: Resource, C: ResourceChecker<R>>(
    &mut self,
    resource: &'a R,
    checker: &'a C,
    stamp: &'a C::Stamp,
  ) -> impl FnOnce(&mut Tracking, Result<Option<&dyn Debug>, &dyn Error>) + 'a {
    self.0.check_resource_start(resource, checker, stamp);
    |tracking, inconsistency| tracking.0.check_resource_end(resource, checker, stamp, inconsistency)
  }

  #[inline]
  #[must_use]
  pub fn execute<'a>(&mut self, task: &'a dyn KeyObj) -> impl FnOnce(&mut Tracking, &dyn ValueObj) + 'a {
    self.0.execute_start(task);
    |tracking, output| tracking.0.execute_end(task, output)
  }

  #[inline]
  #[must_use]
  pub fn schedule_affected_by_task<'a>(
    &mut self,
    task: &'a dyn KeyObj,
  ) -> impl FnOnce(&mut Tracking) + 'a {
    self.0.schedule_affected_by_task_start(task);
    |tracking| tracking.0.schedule_affected_by_task_end(task)
  }
  #[inline]
  #[must_use]
  pub fn check_task_require_task<'a>(
    &mut self,
    requiring_task: &'a dyn KeyObj,
    checker: &'a dyn ValueObj,
    stamp: &'a dyn ValueObj,
  ) -> impl FnOnce(&mut Tracking, Option<&dyn Debug>) + 'a {
    self.0.check_task_require_task_start(requiring_task, checker, stamp);
    |tracking, inconsistency| tracking.0.check_task_require_task_end(requiring_task, checker, stamp, inconsistency)
  }

  #[inline]
  #[must_use]
  pub fn schedule_affected_by_resource<'a>(
    &mut self,
    resource: &'a dyn KeyObj,
  ) -> impl FnOnce(&mut Tracking) + 'a {
    self.0.schedule_affected_by_resource_start(resource);
    |tracking| tracking.0.schedule_affected_by_resource_end(resource)
  }
  #[inline]
  #[must_use]
  pub fn check_task_read_resource<'a>(
    &mut self,
    reading_task: &'a dyn KeyObj,
    checker: &'a dyn ValueObj,
    stamp: &'a dyn ValueObj,
  ) -> impl FnOnce(&mut Tracking, Result<Option<&dyn Debug>, &dyn Error>) + 'a {
    self.0.check_task_read_resource_start(reading_task, checker, stamp);
    |tracking, inconsistency| tracking.0.check_task_read_resource_end(reading_task, checker, stamp, inconsistency)
  }
}
impl<'p> Deref for Tracking<'p> {
  type Target = dyn Tracker + 'p;
  #[inline]
  fn deref(&self) -> &Self::Target { self.0 }
}
impl DerefMut for Tracking<'_> {
  #[inline]
  fn deref_mut(&mut self) -> &mut Self::Target { self.0 }
}
