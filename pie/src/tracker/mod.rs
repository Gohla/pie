use std::error::Error;
use std::fmt::Debug;

use crate::trait_object::{KeyObj, ValueObj};

pub mod writing;
pub mod event;

/// Build event tracker. Can be used to implement logging, event tracing, progress tracking, metrics, etc.
///
/// Object-safe trait.
#[allow(unused_variables)]
pub trait Tracker {
  /// Start: a new build.
  #[inline]
  fn build_start(&mut self) {}
  /// End: completed build.
  #[inline]
  fn build_end(&mut self) {}

  /// Start: require `task` using `checker`.
  #[inline]
  fn require_start(&mut self, task: &dyn KeyObj, checker: &dyn ValueObj) {}
  /// End: required `task`, using `checker` to create `stamp`, resulting in `output`.
  #[inline]
  fn require_end(
    &mut self,
    task: &dyn KeyObj,
    checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    output: &dyn ValueObj,
  ) {}

  /// Start: read `resource` using `checker`.
  #[inline]
  fn read_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj) {}
  /// End: read `resource` using `checker` to create `stamp`.
  #[inline]
  fn read_end(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {}
  /// Start: Write `resource` using `checker`.
  #[inline]
  fn write_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj) {}
  /// End: wrote `resource` using `checker` to create `stamp`.
  #[inline]
  fn write_end(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {}

  /// Start: check consistency of `task` which used `checker` to create `stamp`.
  #[inline]
  fn check_task_start(&mut self, task: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {}
  /// End: checked consistency of `task` which used `checker` to create `stamp`, possibly found an `inconsistency`
  #[inline]
  fn check_task_end(
    &mut self,
    task: &dyn KeyObj,
    checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    inconsistency: Option<&dyn Debug>,
  ) {}

  /// Start: check consistency of `resource` which used `checker` to create `stamp`.
  #[inline]
  fn check_resource_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {}
  /// End: checked consistency of `resource` which used `checker` to create `stamp`, possibly found an `inconsistency`.
  #[inline]
  fn check_resource_end(
    &mut self,
    resource: &dyn KeyObj,
    checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    inconsistency: Result<Option<&dyn Debug>, &dyn Error>,
  ) {}

  /// Start: execute `task`.
  #[inline]
  fn execute_start(&mut self, task: &dyn KeyObj) {}
  /// End: executed `task` resulting in `output`.
  #[inline]
  fn execute_end(&mut self, task: &dyn KeyObj, output: &dyn ValueObj) {}
}

/// Implement [`Tracker`] for `()` that does nothing.
impl Tracker for () {}

/// A [`Tracker`] that forwards events to two [`Tracker`]s.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Debug)]
pub struct CompositeTracker<A1, A2>(pub A1, pub A2);
impl<A1, A2> CompositeTracker<A1, A2> {
  pub fn new(tracker_1: A1, tracker_2: A2) -> Self { Self(tracker_1, tracker_2) }
}
impl<A1: Tracker, A2: Tracker> Tracker for CompositeTracker<A1, A2> {
  #[inline]
  fn build_start(&mut self) {
    self.0.build_start();
    self.1.build_start();
  }
  #[inline]
  fn build_end(&mut self) {
    self.0.build_end();
    self.1.build_end();
  }

  #[inline]
  fn require_start(&mut self, task: &dyn KeyObj, checker: &dyn ValueObj) {
    self.0.require_start(task, checker);
    self.1.require_start(task, checker);
  }
  #[inline]
  fn require_end(
    &mut self,
    task: &dyn KeyObj,
    checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    output: &dyn ValueObj,
  ) {
    self.0.require_end(task, checker, stamp, output);
    self.1.require_end(task, checker, stamp, output);
  }
  #[inline]
  fn read_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj) {
    self.0.read_start(resource, checker);
    self.1.read_start(resource, checker);
  }
  #[inline]
  fn read_end(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {
    self.0.read_end(resource, checker, stamp);
    self.1.read_end(resource, checker, stamp);
  }
  #[inline]
  fn write_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj) {
    self.0.write_start(resource, checker);
    self.1.write_start(resource, checker);
  }
  #[inline]
  fn write_end(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {
    self.0.write_end(resource, checker, stamp);
    self.1.write_end(resource, checker, stamp);
  }

  #[inline]
  fn check_task_start(&mut self, task: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {
    self.0.check_task_start(task, checker, stamp);
    self.1.check_task_start(task, checker, stamp);
  }
  #[inline]
  fn check_task_end(
    &mut self,
    task: &dyn KeyObj,
    checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    inconsistency: Option<&dyn Debug>,
  ) {
    self.0.check_task_end(task, checker, stamp, inconsistency);
    self.1.check_task_end(task, checker, stamp, inconsistency);
  }

  #[inline]
  fn check_resource_start(&mut self, resource: &dyn KeyObj, checker: &dyn ValueObj, stamp: &dyn ValueObj) {
    self.0.check_resource_start(resource, checker, stamp);
    self.1.check_resource_start(resource, checker, stamp);
  }
  #[inline]
  fn check_resource_end(
    &mut self,
    resource: &dyn KeyObj,
    checker: &dyn ValueObj,
    stamp: &dyn ValueObj,
    inconsistency: Result<Option<&dyn Debug>, &dyn Error>,
  ) {
    self.0.check_resource_end(resource, checker, stamp, inconsistency);
    self.1.check_resource_end(resource, checker, stamp, inconsistency);
  }

  #[inline]
  fn execute_start(&mut self, task: &dyn KeyObj) {
    self.0.execute_start(task);
    self.1.execute_start(task);
  }
  #[inline]
  fn execute_end(&mut self, task: &dyn KeyObj, output: &dyn ValueObj) {
    self.0.execute_end(task, output);
    self.1.execute_end(task, output);
  }
}
