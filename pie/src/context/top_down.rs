use std::any::Any;
use std::fmt::Debug;

use crate::{Context, OutputChecker, Resource, ResourceChecker, Task};
use crate::context::SessionExt;
use crate::dependency::{Dependency, TaskDependency};
use crate::pie::SessionInternal;
use crate::store::TaskNode;
use crate::trait_object::ValueEqObj;

/// Top-down incremental context implementation.
///
/// # Implementation Notes
///
/// This cannot have any generic type parameters, as they will propagate into several types and traits, and will end up
/// as generic type parameters of object-safe traits (because object-safe traits cannot have methods with generic type
/// parameters). That will still technically compile, but propagating a generic to [`Dependency`] will mean those
/// dependencies can only be used with a specific instantiation of that generic, which complicates everything.
#[repr(transparent)]
pub struct TopDownContext<'p, 's> {
  session: &'s mut SessionInternal<'p>,
}

impl<'p, 's> TopDownContext<'p, 's> {
  #[inline]
  pub fn new(session: &'s mut SessionInternal<'p>) -> Self { Self { session } }
}

impl Context for TopDownContext<'_, '_> {
  fn require<T, C>(&mut self, task: &T, checker: C) -> T::Output where
    T: Task,
    C: OutputChecker,
  {
    let track_end = self.session.tracker.require(task, &checker);

    let dst = self.session.store.get_or_create_task_node(task);
    self.session.reserve_require_dependency(&dst, task);

    let output = self.make_task_consistent(task);
    let stamp = checker.stamp(&output);
    track_end(&mut self.session.tracker, &stamp, &output);

    self.session.update_require_dependency(&dst, task, checker, stamp);

    output
  }

  #[inline]
  fn read<T, R, C>(&mut self, resource: &T, checker: C) -> Result<R::Reader<'_>, C::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    C: ResourceChecker<R>,
  {
    self.session.read(resource, checker)
  }
  #[inline]
  fn write<T, R, C, F>(&mut self, resource: &T, checker: C, write_fn: F) -> Result<(), C::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    C: ResourceChecker<R>,
    F: FnOnce(&mut R::Writer<'_>) -> Result<(), R::Error>
  {
    self.session.write(resource, checker, write_fn)
  }

  #[inline]
  fn create_writer<'r, R: Resource>(&'r mut self, resource: &'r R) -> Result<R::Writer<'r>, R::Error> {
    self.session.create_writer(resource)
  }
  #[inline]
  fn written_to<T, R, C>(&mut self, resource: &T, checker: C) -> Result<(), C::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    C: ResourceChecker<R>
  {
    self.session.written_to(resource, checker)
  }
}

impl TopDownContext<'_, '_> {
  /// Makes `task` consistent, returning its consistent output.
  #[inline]
  fn make_task_consistent<T: Task>(&mut self, task: &T) -> T::Output {
    let node = self.session.store.get_or_create_task_node(task);

    if self.session.consistent.contains(&node) { // Task is already consistent: return its output.
      return self.session.store.get_task_output(&node)
        .expect("BUG: no task output for already consistent task")
        .as_any().downcast_ref::<T::Output>()
        .expect("BUG: non-matching task output type")
        .clone();
    }

    let output = if let Some(output) = self.check_task::<T::Output>(&node) {
      output.clone()
    } else {
      self.session.store.reset_task(&node);
      let previous_executing_task = self.session.current_executing_task.replace(node);
      let track_end = self.session.tracker.execute(task);
      let output = task.execute(self);
      track_end(&mut self.session.tracker, &output);
      self.session.current_executing_task = previous_executing_task;
      self.session.store.set_task_output(&node, Box::new(output.clone()));
      output
    };

    self.session.consistent.insert(node);
    output
  }

  /// Check whether task `src` is consistent. Returns `Some(output)` when the task is consistent, `None` when
  /// inconsistent. An inconsistent task must be executed.
  ///
  /// A task is consistent if and only if the task adheres to all properties:
  ///
  /// - It is not new. A task is new if it has not been executed before (and thus has no cached output).
  /// - Its output type has not changed.
  /// - All its dependencies are consistent.
  #[inline]
  fn check_task<O: Any>(&mut self, src: &TaskNode) -> Option<&O> {
    let dependencies: Box<[Dependency]> = self.session.store
      .get_dependencies_from_task(src)
      .map(|d| d.clone())
      .collect();
    for dependency in dependencies.into_iter() {
      let consistent = match dependency {
        Dependency::ReservedRequire => panic!("BUG: attempt to consistency check reserved require task dependency"),
        Dependency::Require(d) => Ok(d.as_top_down_check().is_consistent(self)),
        Dependency::Read(d) | Dependency::Write(d) => d.is_consistent_top_down(
          &mut self.session.resource_state,
          &mut self.session.tracker,
        ),
      };
      match consistent {
        Ok(false) => return None,
        Err(e) => {
          self.session.dependency_check_errors.push(e);
          return None;
        }
        _ => {}
      }
    }
    self.session.store.get_task_output(src)
      .map(|o| o.as_any().downcast_ref::<O>().expect("BUG: non-matching task output type"))
  }
}

/// Internal trait for top-down recursive checking of task dependencies.
///
/// Object-safe trait.
pub trait TopDownCheck {
  fn is_consistent(&self, context: &mut TopDownContext) -> bool;
}
impl<T: Task, C: OutputChecker> TopDownCheck for TaskDependency<T, C, Box<dyn ValueEqObj>> {
  #[inline]
  fn is_consistent(&self, context: &mut TopDownContext) -> bool {
    let check_task_end = context.session.tracker.check_task(self.task(), self.checker(), self.stamp());
    let output = context.make_task_consistent(self.task());
    let inconsistency = self.check(&output);
    let inconsistency_dyn = inconsistency.as_ref().map(|o| o as &dyn Debug);
    check_task_end(&mut context.session.tracker, inconsistency_dyn);
    inconsistency.is_none()
  }
}
