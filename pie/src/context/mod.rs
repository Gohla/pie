use crate::{OutputChecker, Resource, ResourceChecker, Task};
use crate::dependency::{Dependency, ResourceDependency, TaskDependency};
use crate::pie::SessionInternal;
use crate::store::{ResourceNode, TaskNode};

pub mod top_down;
pub mod bottom_up;

/// Extension trait on [`SessionInternal`] for usage in [`Context`] implementations.
pub trait SessionExt {
  fn read<T, R, H>(&mut self, resource: &T, checker: H) -> Result<R::Reader<'_>, H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>;
  fn write<T, R, H, F>(&mut self, resource: &T, checker: H, write_fn: F) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
    F: FnOnce(&mut R::Writer<'_>) -> Result<(), R::Error>;

  fn create_writer<'r, R: Resource>(&'r mut self, resource: &'r R) -> Result<R::Writer<'r>, R::Error>;
  fn written_to<T, R, H>(&mut self, resource: &T, checker: H) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>;

  fn reserve_require_dependency<T: Task>(&mut self, dst: &TaskNode, task: &T);
  fn update_require_dependency<T: Task, H: OutputChecker<T::Output>>(&mut self, dst: &TaskNode, task: &T, checker: H, stamp: H::Stamp);
}

impl SessionExt for SessionInternal<'_> {
  fn read<T, R, H>(&mut self, resource: &T, checker: H) -> Result<R::Reader<'_>, H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
  {
    let resource = resource.to_owned();
    let mut reader = resource.read(self.resource_state)
      .map_err(|e| checker.wrap_error(e))?;
    if let Some(current_executing_task_node) = &self.current_executing_task {
      let track_end = self.tracker.read(&resource, &checker);
      let dst = self.store.get_or_create_resource_node(&resource);
      if let Some(writer_node) = self.store.get_task_writing_to_resource(&dst) {
        if !self.store.contains_transitive_task_dependency(current_executing_task_node, &writer_node) {
          let current_executing_task = self.store.get_task(current_executing_task_node);
          let writer_task = self.store.get_task(&writer_node);
          panic!("Hidden dependency; resource '{:?}' is read by the current executing task '{:?}' without a dependency \
                  to the task that writes to it: {:?}", resource, current_executing_task, writer_task);
        }
      }
      let stamp = checker.stamp_reader(&resource, &mut reader)?;
      track_end(&mut self.tracker, &stamp);
      let resource_dependency = ResourceDependency::new(resource, checker, stamp);
      let dependency = Dependency::from_read(resource_dependency);
      let _ = self.store.add_dependency(current_executing_task_node, &dst, dependency);
    };
    Ok(reader)
  }

  fn write<T, R, H, F>(&mut self, resource: &T, checker: H, write_fn: F) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
    F: FnOnce(&mut R::Writer<'_>) -> Result<(), R::Error>,
  {
    let resource = resource.to_owned();
    let dependency_create_inputs = if let Some(current_executing_task_node) = &self.current_executing_task {
      // Validate write before actually writing to the resource, primarily to avoid lifetime issues.
      self.tracker.write_start(&resource, &checker);
      let dst = self.store.get_or_create_resource_node(&resource);
      validate_write(self, &resource, current_executing_task_node, &dst);
      Some((current_executing_task_node, dst))
    } else {
      None
    };

    let mut writer = resource.write(self.resource_state)
      .map_err(|e| checker.wrap_error(e))?;
    write_fn(&mut writer)
      .map_err(|e| checker.wrap_error(e))?;

    if let Some((current_executing_task_node, dst)) = dependency_create_inputs {
      let stamp = checker.stamp_writer(&resource, writer)?;
      self.tracker.write_end(&resource, &checker, &stamp);
      let resource_dependency = ResourceDependency::new(resource, checker, stamp);
      let dependency = Dependency::from_write(resource_dependency);
      let _ = self.store.add_dependency(current_executing_task_node, &dst, dependency);
    }
    Ok(())
  }

  #[inline]
  fn create_writer<'r, R: Resource>(&'r mut self, resource: &'r R) -> Result<R::Writer<'r>, R::Error> {
    resource.write(self.resource_state)
  }

  fn written_to<T, R, H>(&mut self, resource: &T, checker: H) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
  {
    let resource = resource.to_owned();
    if let Some(current_executing_task_node) = &self.current_executing_task {
      let track_end = self.tracker.write(&resource, &checker);
      let dst = self.store.get_or_create_resource_node(&resource);
      validate_write(self, &resource, current_executing_task_node, &dst);
      let stamp = checker.stamp(&resource, self.resource_state)?;
      track_end(&mut self.tracker, &stamp);
      let resource_dependency = ResourceDependency::new(resource, checker, stamp);
      let dependency = Dependency::from_write(resource_dependency);
      let _ = self.store.add_dependency(current_executing_task_node, &dst, dependency);
    };
    Ok(())
  }

  fn reserve_require_dependency<T: Task>(&mut self, dst: &TaskNode, task: &T) {
    if let Some(src) = &self.current_executing_task {
      // Before making the task consistent, first reserve a dependency in the dependency graph, ensuring that all cyclic
      // dependencies are caught before possibly executing a task.
      if let Err(()) = self.store.add_dependency(src, dst, Dependency::ReservedRequire) {
        let src_task = self.store.get_task(src);
        panic!("Cyclic task dependency; current executing task '{:?}' is requiring task '{:?}' which directly or \
            indirectly requires the current executing task", src_task, &task);
      }
    }
  }

  fn update_require_dependency<T: Task, H: OutputChecker<T::Output>>(&mut self, dst: &TaskNode, task: &T, checker: H, stamp: H::Stamp) {
    if let Some(src) = &self.current_executing_task {
      // Update the dependency in the graph from a reserved dependency to a real task require dependency.
      let task_dependency = TaskDependency::new(task.clone(), checker, stamp);
      let dependency = self.store.get_dependency_mut(src, dst);
      *dependency = task_dependency.into();
    }
  }
}

/// Validates a `resource` write from `src` to `dst`, panicking if an overlapping write or hidden dependency was found.
#[inline]
fn validate_write<R: Resource>(session: &SessionInternal<'_>, resource: &R, src: &TaskNode, dst: &ResourceNode) {
  if let Some(previous_writing_task_node) = session.store.get_task_writing_to_resource(dst) {
    let src_task = session.store.get_task(src);
    let previous_writing_task = session.store.get_task(&previous_writing_task_node);
    panic!("Overlapping write; resource '{:?}' is written to by the current executing task '{:?}' that was \
            previously written to by task: {:?}", resource, src_task, previous_writing_task);
  }
  for reading_task_node in session.store.get_tasks_reading_from_resource(dst) {
    if !session.store.contains_transitive_task_dependency(&reading_task_node, src) {
      let src_task = session.store.get_task(src);
      let reading_task = session.store.get_task(&reading_task_node);
      panic!("Hidden dependency; resource '{:?}' is written to by the current executing task '{:?}' without a \
              dependency from reading task '{:?}' to the current executing task", resource, src_task, reading_task);
    }
  }
}
