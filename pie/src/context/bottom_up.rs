use std::collections::hash_map::RandomState;
use std::collections::HashSet;
use std::error::Error;
use std::hash::BuildHasher;

use crate::{Context, OutputChecker, Resource, ResourceChecker, Task};
use crate::context::SessionExt;
use crate::dependency::ResourceDependencyObj;
use crate::pie::{SessionInternal, Tracking};
use crate::store::{Store, TaskNode};
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::base::CloneBox;
use crate::trait_object::collection::TypeToAnyMap;
use crate::trait_object::task::TaskObj;

/// Context that incrementally executes tasks and checks dependencies in a bottom-up manner.
pub struct BottomUpContext<'p, 's> {
  pub(crate) session: &'s mut SessionInternal<'p>,
  scheduled: Queue,
  executing: HashSet<TaskNode>,
}

impl<'p, 's> BottomUpContext<'p, 's> {
  #[inline]
  pub fn new(session: &'s mut SessionInternal<'p>) -> Self {
    Self {
      session,
      scheduled: Queue::new(),
      executing: HashSet::default(),
    }
  }

  /// Schedule tasks affected by `resource`.
  #[inline]
  pub fn schedule_tasks_affected_by(&mut self, resource: &dyn KeyObj) {
    let track_end = self.session.tracker.schedule_affected_by_resource(resource);
    let node = self.session.store.get_or_create_resource_node(resource);
    for (task_node, dependency) in self.session.store.get_read_and_write_dependencies_to_resource(&node) {
      let task = self.session.store.get_task(&task_node);
      Self::try_schedule_task_by_resource_dependency(
        task.as_key_obj(),
        task_node,
        dependency,
        &mut self.session.resource_state,
        &mut self.session.tracker,
        &mut self.session.dependency_check_errors,
        &mut self.scheduled,
        &self.executing,
      );
    }
    track_end(&mut self.session.tracker);
  }

  /// Execute scheduled tasks until queue is empty.
  #[inline]
  pub fn execute_scheduled(&mut self) {
    while let Some(node) = self.scheduled.pop(&mut self.session.store) {
      self.execute_and_schedule(node);
    }
  }

  /// Execute task `node` and potentially schedule new tasks based on the dependencies of the task.
  fn execute_and_schedule(&mut self, node: TaskNode) -> Box<dyn ValueObj> {
    let task = self.session.store.get_task(&node).clone_box();
    let output = self.execute_obj(task.as_ref(), node);

    // Schedule tasks affected by task `node`'s resource writes.
    for written_resource_node in self.session.store.get_resources_written_by(&node) {
      let written_resource = self.session.store.get_resource(&written_resource_node);
      let track_end = self.session.tracker.schedule_affected_by_resource(written_resource);
      // Consider tasks that read `written_resource_node`.
      for (reading_task_node, dependency) in self.session.store.get_read_dependencies_to_resource(&written_resource_node) {
        let reading_task = self.session.store.get_task(&reading_task_node);
        Self::try_schedule_task_by_resource_dependency(
          reading_task.as_key_obj(),
          reading_task_node,
          dependency,
          &mut self.session.resource_state,
          &mut self.session.tracker,
          &mut self.session.dependency_check_errors,
          &mut self.scheduled,
          &self.executing,
        );
      }
      track_end(&mut self.session.tracker);
    }

    // Schedule tasks affected by task `node`'s output.
    let track_end = self.session.tracker.schedule_affected_by_task(task.as_ref().as_key_obj());
    // Consider tasks that require `node`.
    for (requiring_task_node, dependency) in self.session.store.get_require_dependencies_to_task(&node) {
      // TODO: skip when task is already consistent?
      // TODO: skip when task is already scheduled?
      if self.executing.contains(&requiring_task_node) {
        continue; // Don't schedule tasks that are already executing.
      }
      // Note: use `output.as_ref()` instead of `&output`, because `&output` results in a `&Box<dyn ValueObj>` which
      // also implements `dyn ValueObj`, but cannot be downcasted to the concrete unboxed type!
      if !dependency.is_consistent_with(output.as_ref(), &mut self.session.tracker) {
        let requiring_task = self.session.store.get_task(&requiring_task_node);
        self.session.tracker.schedule_task(requiring_task.as_key_obj());
        self.scheduled.add(requiring_task_node);
      }
    }
    track_end(&mut self.session.tracker);

    self.session.consistent.insert(node);
    output
  }

  /// Schedule `task` (with corresponding `node`) if it is affected by a change in its resource `dependency`.
  ///
  /// Note: passing in borrows explicitly instead of a mutable borrow of `self` to make borrows work.
  fn try_schedule_task_by_resource_dependency(
    task: &dyn KeyObj,
    node: TaskNode,
    dependency: &dyn ResourceDependencyObj,
    resource_state: &mut TypeToAnyMap,
    tracker: &mut Tracking,
    dependency_check_errors: &mut Vec<Box<dyn Error>>,
    scheduled: &mut Queue,
    executing: &HashSet<TaskNode>,
  ) {
    // TODO: skip when task is already consistent?
    // TODO: skip when task is already scheduled?
    if executing.contains(&node) {
      return; // Don't schedule tasks that are already executing.
    }
    let consistent = dependency.is_consistent(tracker, resource_state);
    match consistent {
      Err(e) => {
        dependency_check_errors.push(e);
        tracker.schedule_task(task);
        scheduled.add(node);
      }
      Ok(false) => {
        tracker.schedule_task(task);
        scheduled.add(node);
      }
      _ => {}
    }
  }

  /// Execute `task` (with corresponding `node`), returning its result.
  #[inline]
  fn execute<T: Task>(&mut self, task: &T, node: TaskNode) -> T::Output {
    self.session.store.reset_task(&node);
    let previous_executing_task = self.session.current_executing_task.replace(node);
    let track_end = self.session.tracker.execute(task);
    let output = task.execute(self);
    track_end(&mut self.session.tracker, &output);
    self.session.current_executing_task = previous_executing_task;
    self.session.store.set_task_output(&node, Box::new(output.clone()));
    output
  }

  /// Execute `task` as a [task trait object](TaskObj) (with corresponding `node`), returning its result as a boxed
  /// [value trait object](ValueObj).
  #[inline]
  fn execute_obj(&mut self, task: &dyn TaskObj, node: TaskNode) -> Box<dyn ValueObj> {
    self.session.store.reset_task(&node);
    let previous_executing_task = self.session.current_executing_task.replace(node);
    let track_end = self.session.tracker.execute(task.as_key_obj());
    let output = task.execute_bottom_up(self);
    // Note: use `output.as_ref()` instead of `&output`, because `&output` results in a `&Box<dyn ValueObj>` which also
    // implements `dyn ValueObj`, but cannot be downcasted to the concrete unboxed type!
    track_end(&mut self.session.tracker, output.as_ref());
    self.session.current_executing_task = previous_executing_task;
    self.session.store.set_task_output(&node, output.clone());
    output
  }

  /// Execute scheduled tasks (and potentially schedule new tasks) that depend (indirectly) on task `src`, and then
  /// execute `src` if it was scheduled. Returns `Some` output if `task` was (eventually) scheduled and thus executed,
  /// or `None` if `task` was not executed and thus not (eventually) scheduled.
  #[inline]
  fn require_scheduled_now<T: Task>(&mut self, src: &TaskNode) -> Option<T::Output> {
    while self.scheduled.is_not_empty() {
      if let Some(min_task_node) = self.scheduled.pop_least_task_with_dependency_from(src, &self.session.store) {
        let output = self.execute_and_schedule(min_task_node);
        if min_task_node == *src {
          let output = output.into_box_any().downcast::<T::Output>()
            .expect("BUG: non-matching task output type");
          return Some(*output);
        }
      } else {
        break;
      }
    }
    None
  }

  /// Make `task` (with corresponding `node`) consistent, returning its output
  #[inline]
  fn make_task_consistent<T: Task>(&mut self, task: &T, node: TaskNode) -> T::Output {
    if self.session.consistent.contains(&node) { // Task is already consistent: return its output.
      return self.session.store.get_task_output(&node)
        .expect("BUG: no task output for already consistent task")
        .as_any().downcast_ref::<T::Output>()
        .expect("BUG: non-matching task output type")
        .clone();
    }

    if self.session.store.get_task_output(&node).is_none() { // Task is new: execute it.
      return self.execute(task, node);
    }

    // Task is an existing task. Either it has been scheduled if affected, or not scheduled if not affected.
    if let Some(output) = self.require_scheduled_now::<T>(&node) {
      // Task was scheduled. That is, it was either directly or indirectly affected. Therefore, it has been
      // executed, and we return the result of that execution.
      output
    } else {
      // Task was not scheduled. That is, it was not directly affected by resource changes, and not indirectly
      // affected by other tasks.
      //
      // The task cannot be affected during this build. Consider if the task would be affected, this can only occur in
      // 3 different ways:
      //
      // 1. the task is affected by a change in one of its require resource dependencies. But this cannot occur because the
      //    dependency is consistent right now, and cannot become inconsistent due to the absence of hidden dependencies.
      // 2. the task is affected by a change in one of its provided resource dependencies. But this cannot occur because the
      //    dependency is consistent right now, and cannot become inconsistent due to the absence of hidden dependencies
      //    and overlapping provided resources.
      // 3. the task is affected by a change in one of its require task dependencies. But this cannot occur because the
      //    dependency is consistent right now, and cannot become inconsistent because `require_scheduled_now` has made
      //    the task and all its (indirect) dependencies consistent.
      //
      // All case cannot occur, thus the task cannot be affected. Therefore, we don't have to execute the task.
      let output = self.session.store.get_task_output(&node);

      output.expect("BUG: no task output for unaffected task")
        .as_any().downcast_ref::<T::Output>()
        .expect("BUG: non-matching task output type")
        .clone()
    }
  }
}


impl<'p, 's> Context for BottomUpContext<'p, 's> {
  #[inline]
  fn require<T: Task, H: OutputChecker<T::Output>>(&mut self, task: &T, checker: H) -> T::Output {
    let track_end = self.session.tracker.require(task, &checker);

    let dst = self.session.store.get_or_create_task_node(task);
    self.session.reserve_require_dependency(&dst, task);

    let output = self.make_task_consistent(task, dst);
    let stamp = checker.stamp(&output);
    track_end(&mut self.session.tracker, &stamp, &output);

    self.session.update_require_dependency(&dst, task, checker, stamp);

    // Note: make_task_consistent does not insert into self.session.consistent, so do that here.
    self.session.consistent.insert(dst);
    output
  }

  #[inline]
  fn read<T, R, H>(&mut self, resource: &T, checker: H) -> Result<R::Reader<'_>, H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
  {
    self.session.read(resource, checker)
  }
  #[inline]
  fn write<T, R, H, F>(&mut self, resource: &T, checker: H, write_fn: F) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>,
    F: FnOnce(&mut R::Writer<'_>) -> Result<(), R::Error>
  {
    self.session.write(resource, checker, write_fn)
  }

  #[inline]
  fn create_writer<'r, R: Resource>(&'r mut self, resource: &'r R) -> Result<R::Writer<'r>, R::Error> {
    self.session.create_writer(resource)
  }
  #[inline]
  fn written_to<T, R, H>(&mut self, resource: &T, checker: H) -> Result<(), H::Error> where
    T: ToOwned<Owned=R>,
    R: Resource,
    H: ResourceChecker<R>
  {
    self.session.written_to(resource, checker)
  }
}


// Dependency ordered priority queue implementation

#[derive(Default, Debug)]
struct Queue<H = RandomState> {
  set: HashSet<TaskNode, H>,
  vec: Vec<TaskNode>,
}

impl<H: BuildHasher + Default> Queue<H> {
  #[inline]
  fn new() -> Self { Self::default() }

  /// Checks whether the queue is not empty.
  #[inline]
  fn is_not_empty(&self) -> bool { !self.vec.is_empty() }

  /// Add a task to the priority queue. Does nothing if the task is already in the queue.
  #[inline]
  fn add(&mut self, node: TaskNode) {
    if self.set.contains(&node) { return; }
    self.set.insert(node);
    self.vec.push(node);
  }

  /// Remove the last task (task with the least amount of dependencies to other tasks in the queue) from the queue and
  /// return it.
  #[inline]
  fn pop(&mut self, store: &Store) -> Option<TaskNode> {
    self.sort_by_dependencies(store);
    let Some(node) = self.vec.pop() else {
      return None;
    };
    self.set.remove(&node);
    Some(node)
  }

  /// Return the least task (task with the least amount of dependencies to other tasks in the queue) that has a
  /// (transitive) dependency from task `src`.
  #[inline]
  fn pop_least_task_with_dependency_from(&mut self, src: &TaskNode, store: &Store) -> Option<TaskNode> {
    self.sort_by_dependencies(store);
    let mut found = None;
    for (idx, dst) in self.vec.iter().enumerate().rev() {
      if src == dst || store.contains_transitive_task_dependency(src, dst) {
        found = Some((idx, *dst));
        break;
      }
    }
    if let Some((index, task_node_id)) = found {
      self.vec.swap_remove(index); // Note: this prevents allocation but would require resorting as it changes ordering.
      self.set.remove(&task_node_id);
      return Some(task_node_id);
    }
    None
  }

  #[inline]
  fn sort_by_dependencies(&mut self, store: &Store) {
    // TODO: only sort if needed? Removing elements should not require a resort?
    // TODO: use select_nth_unstable_by(0) to get the sorted top element for pop?
    self.vec.sort_unstable_by(|node_a, node_b| store.topologically_compare(node_a, node_b));
  }
}
