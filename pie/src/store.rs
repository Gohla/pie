use std::borrow::Borrow;
use std::collections::HashMap;

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, ResourceDependencyObj, TaskDependencyObj};
use crate::trait_object::{KeyObj, ValueObj};
use crate::trait_object::task::TaskObj;

pub struct Store {
  graph: DAG<NodeData, Dependency>,
  task_to_node: HashMap<Box<dyn TaskObj>, TaskNode>,
  resource_to_node: HashMap<Box<dyn KeyObj>, ResourceNode>,
}

impl Default for Store {
  #[inline]
  fn default() -> Self {
    Self {
      graph: DAG::default(),
      task_to_node: HashMap::default(),
      resource_to_node: HashMap::default(),
    }
  }
}

enum NodeData {
  Resource(Box<dyn KeyObj>),
  Task {
    task: Box<dyn TaskObj>,
    output: Option<Box<dyn ValueObj>>,
  },
}

/// Newtype for task [`Node`]s.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct TaskNode(Node);
impl Borrow<Node> for &TaskNode {
  fn borrow(&self) -> &Node { &self.0 }
}

/// Newtype for resource [`Node`]s.
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct ResourceNode(Node);
impl Borrow<Node> for &ResourceNode {
  fn borrow(&self) -> &Node { &self.0 }
}

impl Store {
  /// Gets the task node for `task`, or creates a task node by adding it to the dependency graph.
  #[inline]
  pub fn get_or_create_task_node(&mut self, task: &dyn TaskObj) -> TaskNode {
    if let Some(node) = self.task_to_node.get(task) {
      *node
    } else {
      let node = self.graph.add_node(NodeData::Task {
        task: task.to_owned(),
        output: None,
      });
      let node = TaskNode(node);
      self.task_to_node.insert(task.to_owned(), node);
      node
    }
  }
  /// Gets the task for `node`.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph.
  #[inline]
  pub fn get_task(&self, node: &TaskNode) -> &dyn TaskObj {
    let Some(NodeData::Task { task, .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: {:?} was not found in the dependency graph", node);
    };
    task.as_ref()
  }


  /// Gets the resource node for `resource`, or creates a resource node by adding it to the dependency graph.
  #[inline]
  pub fn get_or_create_resource_node(&mut self, resource: &dyn KeyObj) -> ResourceNode {
    if let Some(node) = self.resource_to_node.get(resource) {
      *node
    } else {
      let node = self.graph.add_node(NodeData::Resource(resource.to_owned()));
      let node = ResourceNode(node);
      self.resource_to_node.insert(resource.to_owned(), node);
      node
    }
  }
  /// Gets the task for `node`.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph.
  #[inline]
  pub fn get_resource(&self, node: &ResourceNode) -> &dyn KeyObj {
    let Some(NodeData::Resource(resource)) = self.graph.get_node_data(node) else {
      panic!("BUG: {:?} was not found in the dependency graph", node);
    };
    resource.as_ref()
  }


  /// Gets the output for task `node`.
  ///
  /// # Panics
  ///
  /// Panics if `node` was not found in the dependency graph.
  #[inline]
  pub fn get_task_output(&self, node: &TaskNode) -> Option<&dyn ValueObj> {
    let Some(NodeData::Task { output, .. }) = self.graph.get_node_data(node) else {
      panic!("BUG: {:?} was not found in the dependency graph", node);
    };
    output.as_ref().map(|o| o.as_ref())
  }
  /// Sets the output for task `node` to `new_output`.
  ///
  /// # Panics
  ///
  /// Panics if task `node` was not found in the dependency graph.
  #[inline]
  pub fn set_task_output(&mut self, node: &TaskNode, new_output: Box<dyn ValueObj>) {
    let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(node) else {
      panic!("BUG: {:?} was not found in the dependency graph", node);
    };
    // OPTO: try to clone output into existing allocation for output. Also requires `reset_task` to not remove that.
    output.replace(new_output);
  }

  /// Compare task `node_a` and  task `node_b`, topographically.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `node_a` or `node_b` were not found in the dependency graph.
  #[inline]
  pub fn topologically_compare(&self, node_a: &TaskNode, node_b: &TaskNode) -> std::cmp::Ordering {
    // TODO: test
    debug_assert!(self.graph.contains_node(node_a), "BUG: {:?} was not found in the dependency graph", node_a);
    debug_assert!(self.graph.contains_node(node_b), "BUG: {:?} was not found in the dependency graph", node_b);
    self.graph.topo_cmp(node_a, node_b)
  }
  /// Checks whether there is a direct or transitive dependency from task `src` to task `dst`. Returns false when either
  /// node was not found in the dependency graph.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `src` or `dst` were not found in the dependency graph.
  #[inline]
  pub fn contains_transitive_task_dependency(&self, src: &TaskNode, dst: &TaskNode) -> bool {
    debug_assert!(self.graph.contains_node(src), "BUG: {:?} was not found in the dependency graph", src);
    debug_assert!(self.graph.contains_node(dst), "BUG: {:?} was not found in the dependency graph", dst);
    self.graph.contains_transitive_edge(src, dst)
  }
  /// Get all task nodes that read from resource `dst`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `dst` was not found in the dependency graph.
  #[inline]
  pub fn get_tasks_reading_from_resource<'a>(&'a self, dst: &'a ResourceNode) -> impl Iterator<Item=TaskNode> + 'a {
    debug_assert!(self.graph.contains_node(dst), "BUG: {:?} was not found in the dependency graph", dst);
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| matches!(d, Dependency::Read(_)).then(|| TaskNode(*n)))
  }
  /// Get the task node that writes to resource `dst`, or `None` if there is none.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `dst` was not found in the dependency graph.
  #[inline]
  pub fn get_task_writing_to_resource(&self, dst: &ResourceNode) -> Option<TaskNode> {
    debug_assert!(self.graph.contains_node(dst), "BUG: {:?} was not found in the dependency graph", dst);
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| matches!(d, Dependency::Write(_)).then(|| TaskNode(*n)))
      .next()
  }
  /// Get all task nodes and corresponding dependencies that read from to resource `dst`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `dst` was not found in the dependency graph.
  #[inline]
  pub fn get_read_dependencies_to_resource<'a>(&'a self, dst: &'a ResourceNode) -> impl Iterator<Item=(TaskNode, &dyn ResourceDependencyObj)> + 'a {
    debug_assert!(self.graph.contains_node(dst), "BUG: {:?} was not found in the dependency graph", dst);
    self.graph.get_incoming_edges(dst).filter_map(|(n, d)| match d {
      Dependency::Read(rd) => Some((TaskNode(*n), rd.as_ref())),
      _ => None,
    })
  }
  /// Get all task nodes and corresponding dependencies that read from or write to resource `dst`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `dst` was not found in the dependency graph.
  #[inline]
  pub fn get_read_and_write_dependencies_to_resource<'a>(&'a self, dst: &'a ResourceNode) -> impl Iterator<Item=(TaskNode, &dyn ResourceDependencyObj)> + 'a {
    debug_assert!(self.graph.contains_node(dst), "BUG: {:?} was not found in the dependency graph", dst);
    self.graph.get_incoming_edges(dst).filter_map(|(n, d)| match d {
      Dependency::Read(rd) | Dependency::Write(rd) => Some((TaskNode(*n), rd.as_ref())),
      _ => None,
    })
  }
  /// Get all dependencies from task `src`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `src` was not found in the dependency graph.
  pub fn get_dependencies_from_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Dependency> + 'a {
    debug_assert!(self.graph.contains_node(src), "BUG: {:?} was not found in the dependency graph", src);
    self.graph.get_outgoing_edge_data(src)
  }
  /// Get all task nodes and corresponding require dependencies to task `dst`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `dst` was not found in the dependency graph.
  #[inline]
  pub fn get_require_dependencies_to_task<'a>(&'a self, dst: &'a TaskNode) -> impl Iterator<Item=(TaskNode, &dyn TaskDependencyObj)> + 'a {
    debug_assert!(self.graph.contains_node(dst), "BUG: {:?} was not found in the dependency graph", dst);
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| match d {
        Dependency::Require(td) => Some((TaskNode(*n), td.as_ref())),
        _ => None
      })
  }

  /// Get all resource nodes that are written by task `src`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `src` was not found in the dependency graph.
  #[inline]
  pub fn get_resources_written_by<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=ResourceNode> + 'a {
    debug_assert!(self.graph.contains_node(src), "BUG: {:?} was not found in the dependency graph", src);
    self.graph.get_outgoing_edges(src)
      .filter_map(|(n, d)| matches!(d, Dependency::Write(_)).then(|| ResourceNode(*n)))
  }

  /// Adds a `dependency` from `src` to `dst`.
  ///
  /// # Errors
  ///
  /// Returns an error if a cycle is created by adding this dependency.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` was not found in the dependency graph.
  pub fn add_dependency<'a>(&mut self, src: impl Borrow<Node>, dst: impl Borrow<Node>, dependency: Dependency) -> Result<(), ()> {
    let src = src.borrow();
    let dst = dst.borrow();
    match self.graph.add_edge(src, dst, dependency) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source {:?} and/or destination {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => Err(()),
      _ => Ok(()),
    }
  }
  /// Gets the mutable `dependency` from `src` to `dst`.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph, or if the dependency from `src` to `dst` was not
  /// found in the dependency graph.
  #[inline]
  pub fn get_dependency_mut(&mut self, src: impl Borrow<Node>, dst: impl Borrow<Node>) -> &mut Dependency {
    let src = src.borrow();
    let dst = dst.borrow();
    let Some(dependency) = self.graph.get_edge_data_mut(src, dst) else {
      panic!("BUG: no task dependency was found between source {:?} and destination {:?}", src, dst)
    };
    dependency
  }


  /// Reset task `src`, removing its output and removing all its outgoing dependencies.
  ///
  /// # Panics
  ///
  /// Panics if task `src` was not found in the dependency graph.
  #[inline]
  pub fn reset_task(&mut self, src: &TaskNode) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(src) {
      *output = None;
    } else {
      panic!("BUG: {:?} was not found in the dependency graph", src);
    }
    self.graph.remove_outgoing_edges_of_node(src);
  }
}


#[cfg(test)]
mod test {
  use std::path::PathBuf;

  use assert_matches::assert_matches;

  use dev_util::downcast_ref_or_panic;

  use crate::Context;
  use crate::dependency::{ResourceDependency, TaskDependency};
  use crate::resource::file::{ExistsChecker, ModifiedChecker};
  use crate::Task;
  use crate::task::EqualsChecker;

  use super::*;

  /// Implement [`Task`] for string literals.
  impl Task for &'static str {
    type Output = &'static str;
    fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
      self
    }
  }

  /// Cast trait objects to types used in tests.
  trait Cast {
    fn as_path(&self) -> &PathBuf;
    fn as_str(&self) -> &'static str;
  }
  impl Cast for dyn KeyObj {
    fn as_path(&self) -> &PathBuf {
      downcast_ref_or_panic(self.as_any())
    }
    fn as_str(&self) -> &'static str {
      downcast_ref_or_panic::<&'static str>(self.as_any())
    }
  }
  impl Cast for dyn ValueObj {
    fn as_path(&self) -> &PathBuf {
      downcast_ref_or_panic(self.as_any())
    }
    fn as_str(&self) -> &'static str {
      downcast_ref_or_panic::<&'static str>(self.as_any())
    }
  }
  impl Cast for dyn TaskObj {
    fn as_path(&self) -> &PathBuf {
      downcast_ref_or_panic(self.as_any())
    }
    fn as_str(&self) -> &'static str {
      downcast_ref_or_panic::<&'static str>(self.as_any())
    }
  }


  #[test]
  fn test_resource_mapping() {
    let mut store: Store = Store::default();

    let path_a = PathBuf::from("hello.txt");
    let node_a = store.get_or_create_resource_node(&path_a);
    assert_eq!(node_a, store.get_or_create_resource_node(&path_a)); // Same node
    assert_eq!(&path_a, store.get_resource(&node_a).as_path()); // Same resource path

    let path_b = PathBuf::from("world.txt");
    let node_b = store.get_or_create_resource_node(&path_b);
    assert_eq!(node_b, store.get_or_create_resource_node(&path_b));
    assert_eq!(&path_b, store.get_resource(&node_b).as_path());

    assert_ne!(node_a, node_b); // Different nodes
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_resource_mapping_panics() {
    let mut fake_store: Store = Store::default();
    let fake_node = fake_store.get_or_create_resource_node(&PathBuf::from("hello.txt"));
    let store: Store = Store::default();
    store.get_resource(&fake_node);
  }


  #[test]
  fn test_task_mapping() {
    let mut store = Store::default();

    let task_a = "Hello";
    let node_a = store.get_or_create_task_node(&task_a);
    assert_eq!(node_a, store.get_or_create_task_node(&task_a)); // Same node
    assert_eq!(task_a, store.get_task(&node_a).as_str()); // Same task

    let task_b = "World";
    let node_b = store.get_or_create_task_node(&task_b);
    assert_eq!(node_b, store.get_or_create_task_node(&task_b));
    assert_eq!(task_b, store.get_task(&node_b).as_str());

    assert_ne!(node_a, node_b); // Different nodes
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_task_mapping_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&"Hello");
    let store: Store = Store::default();
    store.get_task(&fake_node);
  }

  #[test]
  fn test_task_outputs() {
    let mut store = Store::default();
    let output_a = "Hello";
    let task_a = output_a;
    let node_a = store.get_or_create_task_node(&task_a);

    let output_b = "World";
    let task_b = output_b;
    let node_b = store.get_or_create_task_node(&task_b);

    // Assert that tasks have no output by default.
    assert_matches!(store.get_task_output(&node_a), None);
    assert_matches!(store.get_task_output(&node_b), None);

    // Set output for task A, assert that A has that output but B is unchanged.
    store.set_task_output(&node_a, Box::new(output_a));
    assert_eq!(store.get_task_output(&node_a).map(|v| v.as_str()), Some(output_a));
    assert_matches!(store.get_task_output(&node_b), None);

    // Set output for task B, assert that B has that output but A is unchanged.
    store.set_task_output(&node_b, Box::new(output_b));
    assert_eq!(store.get_task_output(&node_a).map(|v| v.as_str()), Some(output_a));
    assert_eq!(store.get_task_output(&node_b).map(|v| v.as_str()), Some(output_b));
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_get_task_output_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&"Hello");
    let store = Store::default();
    store.get_task_output(&fake_node);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_set_task_output_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&"Hello");
    let mut store: Store = Store::default();
    store.set_task_output(&fake_node, Box::new("Hello"));
  }


  #[test]
  fn test_dependencies() {
    let mut store = Store::default();
    let output_a = "Hello";
    let task_a = output_a;
    let node_a = store.get_or_create_task_node(&task_a);
    let output_b = "World";
    let task_b = output_b;
    let node_b = store.get_or_create_task_node(&task_b);
    let path_c = PathBuf::from("hello.txt");
    let node_c = store.get_or_create_resource_node(&path_c);

    assert!(!store.contains_transitive_task_dependency(&node_a, &node_a));
    assert!(!store.contains_transitive_task_dependency(&node_b, &node_b));
    assert!(!store.contains_transitive_task_dependency(&node_a, &node_b));
    assert!(!store.contains_transitive_task_dependency(&node_b, &node_a));
    assert_eq!(store.get_tasks_reading_from_resource(&node_c).next(), None);
    assert_eq!(store.get_task_writing_to_resource(&node_c), None);
    assert_matches!(store.get_read_and_write_dependencies_to_resource(&node_c).next(), None);
    assert_matches!(store.get_read_dependencies_to_resource(&node_c).next(), None);
    assert_eq!(store.get_dependencies_from_task(&node_a).next(), None);
    assert_eq!(store.get_dependencies_from_task(&node_b).next(), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_a).next(), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_b).next(), None);
    assert_eq!(store.get_resources_written_by(&node_a).next(), None);
    assert_eq!(store.get_resources_written_by(&node_b).next(), None);

    // Add resource read dependency from task A to resource C.
    let read_a2c = ResourceDependency::new(path_c.clone(), ModifiedChecker, None).into_read();
    let result = store.add_dependency(&node_a, &node_c, read_a2c.clone());
    assert_eq!(result, Ok(()));
    assert!(!store.contains_transitive_task_dependency(&node_a, &node_b));
    assert!(!store.contains_transitive_task_dependency(&node_b, &node_a));
    let reads_from_c: Vec<_> = store.get_tasks_reading_from_resource(&node_c).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    assert_eq!(store.get_task_writing_to_resource(&node_c), None);
    let reads_from_c: Vec<_> = store.get_read_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    let reads_or_writes_from_c: Vec<_> = store.get_read_and_write_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_or_writes_from_c.get(0), Some(&node_a));
    assert_eq!(reads_or_writes_from_c.get(1), None);
    let deps_of_a: Vec<_> = store.get_dependencies_from_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&read_a2c));
    assert_eq!(deps_of_a.get(1), None);
    assert_eq!(store.get_dependencies_from_task(&node_b).next(), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_a).next(), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_b).next(), None);
    assert_eq!(store.get_resources_written_by(&node_a).next(), None);
    assert_eq!(store.get_resources_written_by(&node_b).next(), None);

    // Reserve task dependency from task B to task A.
    let result = store.add_dependency(&node_b, &node_a, Dependency::ReservedRequire);
    assert_eq!(result, Ok(()));
    assert!(!store.contains_transitive_task_dependency(&node_a, &node_b));
    assert!(store.contains_transitive_task_dependency(&node_b, &node_a));
    let reads_from_c: Vec<_> = store.get_tasks_reading_from_resource(&node_c).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    assert_eq!(store.get_task_writing_to_resource(&node_c), None);
    let reads_from_c: Vec<_> = store.get_read_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    let reads_or_writes_from_c: Vec<_> = store.get_read_and_write_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_or_writes_from_c.get(0), Some(&node_a));
    assert_eq!(reads_or_writes_from_c.get(1), None);
    let deps_of_a: Vec<_> = store.get_dependencies_from_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&read_a2c));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_from_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&Dependency::ReservedRequire));
    assert_eq!(deps_of_b.get(1), None);
    // Note: still None because get_require_dependencies_to_task correctly does not match ReservedRequire.
    assert_matches!(store.get_require_dependencies_to_task(&node_a).next(), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_b).next(), None);
    assert_matches!(store.get_resources_written_by(&node_a).next(), None);
    assert_matches!(store.get_resources_written_by(&node_b).next(), None);

    // Update task dependency from task B to task A.
    let require_b2a = TaskDependency::from_typed(task_a, EqualsChecker, Box::new(output_a)).into_require();
    *store.get_dependency_mut(&node_b, &node_a) = require_b2a.clone();
    assert!(!store.contains_transitive_task_dependency(&node_a, &node_b));
    assert!(store.contains_transitive_task_dependency(&node_b, &node_a));
    let reads_from_c: Vec<_> = store.get_tasks_reading_from_resource(&node_c).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    assert_eq!(store.get_task_writing_to_resource(&node_c), None);
    let reads_from_c: Vec<_> = store.get_read_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    let reads_or_writes_from_c: Vec<_> = store.get_read_and_write_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_or_writes_from_c.get(0), Some(&node_a));
    assert_eq!(reads_or_writes_from_c.get(1), None);
    let deps_of_a: Vec<_> = store.get_dependencies_from_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&read_a2c));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_from_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&require_b2a));
    assert_eq!(deps_of_b.get(1), None);
    let reqs_to_a: Vec<_> = store.get_require_dependencies_to_task(&node_a).map(|(n, _)| n).collect();
    assert_eq!(reqs_to_a.get(0), Some(&node_b));
    assert_eq!(reqs_to_a.get(1), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_b).next(), None);
    assert_matches!(store.get_resources_written_by(&node_a).next(), None);
    assert_matches!(store.get_resources_written_by(&node_b).next(), None);

    // Add resource write dependency from task B to resource C.
    let write_b2c = ResourceDependency::new(path_c.clone(), ExistsChecker, true).into_write();
    let result = store.add_dependency(&node_b, &node_c, write_b2c.clone());
    assert_eq!(result, Ok(()));
    assert!(!store.contains_transitive_task_dependency(&node_a, &node_b));
    assert!(store.contains_transitive_task_dependency(&node_b, &node_a));
    let reads_from_c: Vec<_> = store.get_tasks_reading_from_resource(&node_c).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    assert_eq!(store.get_task_writing_to_resource(&node_c), Some(node_b));
    let reads_from_c: Vec<_> = store.get_read_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_from_c.get(0), Some(&node_a));
    assert_eq!(reads_from_c.get(1), None);
    let reads_or_writes_from_c: Vec<_> = store.get_read_and_write_dependencies_to_resource(&node_c).map(|(n, _)| n).collect();
    assert_eq!(reads_or_writes_from_c.get(0), Some(&node_a));
    assert_eq!(reads_or_writes_from_c.get(1), Some(&node_b));
    assert_eq!(reads_or_writes_from_c.get(2), None);
    let deps_of_a: Vec<_> = store.get_dependencies_from_task(&node_a).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&read_a2c));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_from_task(&node_b).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&require_b2a));
    assert_eq!(deps_of_b.get(1), Some(&write_b2c));
    assert_eq!(deps_of_b.get(2), None);
    let reqs_to_a: Vec<_> = store.get_require_dependencies_to_task(&node_a).map(|(n, _)| n).collect();
    assert_eq!(reqs_to_a.get(0), Some(&node_b));
    assert_eq!(reqs_to_a.get(1), None);
    assert_matches!(store.get_require_dependencies_to_task(&node_b).next(), None);
    assert_matches!(store.get_resources_written_by(&node_a).next(), None);
    let writes_from_b: Vec<_> = store.get_resources_written_by(&node_b).collect();
    assert_eq!(writes_from_b.get(0), Some(&node_c));
    assert_eq!(writes_from_b.get(1), None);

    // Reserve task dependency from task A to task B, creating a cycle.
    let result = store.add_dependency(&node_a, &node_b, Dependency::ReservedRequire);
    assert_eq!(result, Err(())); // Creates a cycle: error
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_contains_transitive_task_dependency_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&"Hello");
    let store = Store::default();
    let _ = store.contains_transitive_task_dependency(&fake_node, &fake_node);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn get_tasks_reading_from_panics() {
    let path = PathBuf::from("hello.txt");
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_resource_node(&path);
    let store = Store::default();
    let _ = store.get_tasks_reading_from_resource(&fake_node);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn get_task_writing_to_panics() {
    let path = PathBuf::from("hello.txt");
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_resource_node(&path);
    let store = Store::default();
    let _ = store.get_task_writing_to_resource(&fake_node);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_get_dependencies_of_task_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&"Hello");
    let store = Store::default();
    let _ = store.get_dependencies_from_task(&fake_node);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_add_resource_dependency_panics() {
    let path = PathBuf::from("hello.txt");
    let mut fake_store = Store::default();
    let fake_resource_node = fake_store.get_or_create_resource_node(&path);
    let fake_task_node = fake_store.get_or_create_task_node(&"Hello");
    let mut store: Store = Store::default();
    let dependency = ResourceDependency::new(path, ExistsChecker, true).into_read();
    let _ = store.add_dependency(&fake_task_node, &fake_resource_node, dependency);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_add_task_dependency_panics() {
    let output = "Hello";
    let task = output;
    let mut fake_store = Store::default();
    let fake_task_node = fake_store.get_or_create_task_node(&task);
    let mut store: Store = Store::default();
    let dependency = TaskDependency::from_typed(task, EqualsChecker, Box::new(output)).into_require();
    let _ = store.add_dependency(&fake_task_node, &fake_task_node, dependency);
  }


  #[test]
  fn test_reset() {
    let mut store = Store::default();
    let output_a = "Hello";
    let task_a = output_a;
    let task_a_node = store.get_or_create_task_node(&task_a);
    let output_b = "World";
    let task_b = output_b;
    let task_b_node = store.get_or_create_task_node(&task_b);
    let path = PathBuf::from("hello.txt");
    let resource_node = store.get_or_create_resource_node(&path);

    // Set outputs for task A and B.
    store.set_task_output(&task_a_node, Box::new(output_a));
    assert_eq!(store.get_task_output(&task_a_node).map(|v| v.as_str()), Some(output_a));
    store.set_task_output(&task_b_node, Box::new(output_b));
    assert_eq!(store.get_task_output(&task_b_node).map(|v| v.as_str()), Some(output_b));

    // Add resource read dependency from task A and B.
    let read_dep = ResourceDependency::new(path, ExistsChecker, true).into_read();
    let result = store.add_dependency(&task_a_node, &resource_node, read_dep.clone());
    assert_eq!(result, Ok(()));
    let result = store.add_dependency(&task_b_node, &resource_node, read_dep.clone());
    assert_eq!(result, Ok(()));
    let reads: Vec<_> = store.get_tasks_reading_from_resource(&resource_node).collect();
    assert_eq!(reads.get(0), Some(&task_a_node));
    assert_eq!(reads.get(1), Some(&task_b_node));
    assert_eq!(reads.get(2), None);
    assert_eq!(store.get_task_writing_to_resource(&resource_node), None);
    let reads: Vec<_> = store.get_read_dependencies_to_resource(&resource_node).map(|(n, _)| n).collect();
    assert_eq!(reads.get(0), Some(&task_a_node));
    assert_eq!(reads.get(1), Some(&task_b_node));
    assert_eq!(reads.get(2), None);
    let reads_or_writes: Vec<_> = store.get_read_and_write_dependencies_to_resource(&resource_node).map(|(n, _)| n).collect();
    assert_eq!(reads_or_writes.get(0), Some(&task_a_node));
    assert_eq!(reads_or_writes.get(1), Some(&task_b_node));
    assert_eq!(reads_or_writes.get(2), None);
    let deps_of_a: Vec<_> = store.get_dependencies_from_task(&task_a_node).cloned().collect();
    assert_eq!(deps_of_a.get(0), Some(&read_dep));
    assert_eq!(deps_of_a.get(1), None);
    let deps_of_b: Vec<_> = store.get_dependencies_from_task(&task_b_node).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&read_dep));
    assert_eq!(deps_of_b.get(1), None);

    // Reset only task A.
    store.reset_task(&task_a_node);
    // Assert that task A is reset.
    assert_matches!(store.get_task_output(&task_a_node), None);
    let reads: Vec<_> = store.get_tasks_reading_from_resource(&resource_node).collect();
    assert_eq!(reads.get(0), Some(&task_b_node));
    assert_eq!(reads.get(1), None);
    let reads: Vec<_> = store.get_read_dependencies_to_resource(&resource_node).map(|(n, _)| n).collect();
    assert_eq!(reads.get(0), Some(&task_b_node));
    assert_eq!(reads.get(1), None);
    let reads_or_writes: Vec<_> = store.get_read_and_write_dependencies_to_resource(&resource_node).map(|(n, _)| n).collect();
    assert_eq!(reads_or_writes.get(0), Some(&task_b_node));
    assert_eq!(reads_or_writes.get(1), None);
    assert_eq!(store.get_dependencies_from_task(&task_a_node).next(), None);
    // Assert that task B is unchanged.
    assert_eq!(store.get_task_output(&task_b_node).map(|v| v.as_str()), Some(output_b));
    let deps_of_b: Vec<_> = store.get_dependencies_from_task(&task_b_node).cloned().collect();
    assert_eq!(deps_of_b.get(0), Some(&read_dep));
    assert_eq!(deps_of_b.get(1), None);
  }

  #[test]
  #[should_panic(expected = "was not found in the dependency graph")]
  fn test_reset_task_panics() {
    let mut fake_store = Store::default();
    let fake_node = fake_store.get_or_create_task_node(&"Hello");
    let mut store: Store = Store::default();
    store.reset_task(&fake_node);
  }
}
