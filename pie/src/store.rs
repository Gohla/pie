use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::{Path, PathBuf};

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency, TaskDependency};
use crate::Task;

pub type TaskNode = Node;
pub type FileNode = Node;

#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(bound(serialize = "T: serde::Serialize + Task, O: serde::Serialize, H: BuildHasher + Default")))]
#[cfg_attr(feature = "serde", serde(bound(deserialize = "T: serde::Deserialize<'de> + Task, O: serde::Deserialize<'de>, H: BuildHasher + Default")))]
pub struct Store<T, O, H> {
  graph: DAG<NodeData<T, O>, Option<Dependency<T, O>>, H>,
  task_to_node: HashMap<T, TaskNode, H>,
  file_to_node: HashMap<PathBuf, FileNode, H>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NodeData<T, O> {
  Task {
    task: T,
    output: Option<O>,
  },
  File(PathBuf),
}

impl<T: Task, H: BuildHasher + Default> Default for Store<T, T::Output, H> {
  #[inline]
  fn default() -> Self {
    Self {
      graph: DAG::with_default_hasher(),
      task_to_node: HashMap::default(),
      file_to_node: HashMap::default(),
    }
  }
}

impl<T: Task, H: BuildHasher + Default> Store<T, T::Output, H> {
  /// Gets the node for `task`, or creates a node by adding it to the dependency graph.
  #[inline]
  pub fn get_or_create_task_node(&mut self, task: &T) -> TaskNode {
    if let Some(node) = self.task_to_node.get(task) {
      *node
    } else {
      let node = self.graph.add_node(NodeData::Task {
        task: task.clone(),
        output: None,
      });
      self.task_to_node.insert(task.clone(), node);
      node
    }
  }
  /// Gets the task for `node`, panicking if `node` was not found in the dependency graph or if `node` is a file.
  #[inline]
  pub fn get_task(&self, node: &TaskNode) -> &T {
    let data = self.graph.get_node_data(node)
      .expect("node should exist in dependency graph");
    match data {
      NodeData::Task { task, .. } => task,
      _ => panic!("node should be a task node")
    }
  }
  
  
  /// Gets the file node for `path`, or creates a node by adding it to the dependency graph.
  #[inline]
  pub fn get_or_create_file_node(&mut self, path: impl AsRef<Path>) -> FileNode {
    let path = path.as_ref();
    if let Some(file_node) = self.file_to_node.get(path) {
      *file_node
    } else {
      let node = self.graph.add_node(NodeData::File(path.to_path_buf()));
      self.file_to_node.insert(path.to_path_buf(), node);
      node
    }
  }
  /// Gets the path for `node`, panicking if `node` was not found in the dependency graph or if `node` is a task.
  #[inline]
  pub fn get_file_path(&self, node: &FileNode) -> &PathBuf {
    let data = self.graph.get_node_data(node)
      .expect("node should exist in dependency graph");
    match data {
      NodeData::File(path) => path,
      _ => panic!("node should be a file node")
    }
  }


  /// Checks whether task `node` has an output. Returns `false` if `node` does not have an output , if `node` does not
  /// exist in the dependency graph, or if `node` is a file.
  #[inline]
  pub fn task_has_output(&self, node: &TaskNode) -> bool {
    if let Some(NodeData::Task { output: Some(_), .. }) = self.graph.get_node_data(node) {
      true
    } else {
      false
    }
  }
  /// Gets the output for task `node`, panicking if `node` was not found in the dependency graph, if `node` is a file,
  /// or if the task has no output.
  #[inline]
  pub fn get_task_output(&self, node: &TaskNode) -> &T::Output {
    let data = self.graph.get_node_data(node)
      .expect("node should exist in dependency graph");
    match data {
      NodeData::Task { output, .. } => output.as_ref().expect("task should have an output"),
      _ => panic!("node should be a task node")
    }
  }
  /// Sets the output for task `node` to `new_output`. Does nothing if task `node` does not exist in the dependency 
  /// graph.
  #[inline]
  pub fn set_task_output(&mut self, node: &TaskNode, new_output: T::Output) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(node) {
      output.replace(new_output);
    }
  }


  /// Get all destination nodes of the outgoing dependencies of task `src`.
  #[inline]
  pub fn get_dependencies_of_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Node> + 'a {
    self.graph.get_outgoing_edge_nodes(src)
  }
  /// Get the dependency between `src` and `dst`. Returns `None` if there is no dependency between `src` and `dst`, or 
  /// when the dependency exists but is a reserved task dependency.
  #[inline]
  pub fn get_dependency(&self, src: &Node, dst: &Node) -> Option<&Dependency<T, T::Output>> {
    self.graph.get_edge_data(src, dst).unwrap_or(&None).as_ref()
  }
  /// Compare `src` and `dst`, topographically.
  #[inline]
  pub fn topologically_compare(&self, node_a: &Node, node_b: &Node) -> std::cmp::Ordering {
    self.graph.topo_cmp(node_a, node_b)
  }


  /// Add a file require dependency from task `src` to file `dst`.
  #[inline]
  pub fn add_file_require_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    // OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.graph.add_edge(src, dst, Some(Dependency::RequireFile(dependency))).ok();
  }
  /// Add a file provide dependency from task `src` to file `dst`.
  #[inline]
  pub fn add_file_provide_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    // OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.graph.add_edge(src, dst, Some(Dependency::ProvideFile(dependency))).ok();
  }

  /// Reserve a task require dependency from task `src` to task `dst`. Returns an `Err` if this dependency creates a 
  /// cycle. This reservation is required because the dependency from `src` and `dst` should already exist for 
  /// validation purposes (cycles, hidden dependencies, etc.), but we do not yet have the output of task `dst` so we
  /// cannot fully create the task dependency.
  #[inline]
  pub fn reserve_task_require_dependency(&mut self, src: &TaskNode, dst: &Node) -> Result<(), pie_graph::Error> {
    self.graph.add_edge(src, dst, None)?;
    Ok(())
  }
  /// Update the reserved task require dependency from task `src` to `dst` to `dependency`. Panics if the dependency was
  /// not reserved before.
  #[inline]
  pub fn update_reserved_task_require_dependency(&mut self, src: &TaskNode, dst: &Node, dependency: TaskDependency<T, T::Output>) {
    self.graph.get_edge_data_mut(src, dst).unwrap().replace(Dependency::RequireTask(dependency));
  }


  /// Reset task `src`, removing all its dependencies.
  #[inline]
  pub fn reset_task(&mut self, src: &TaskNode) {
    self.graph.remove_edges_of_node(src);
    // TODO: should this remove output?
  }


  /// Checks whether there is a direct or transitive dependency from `src` to `dst`.
  #[inline]
  pub fn contains_transitive_task_dependency(&self, src: &TaskNode, dst: &TaskNode) -> bool {
    self.graph.contains_transitive_edge(src, dst)
  }


  /// Get all task nodes and corresponding dependencies that require task `dst`.
  #[inline]
  pub fn get_tasks_requiring_task<'a>(&'a self, dst: &'a TaskNode) -> impl Iterator<Item=(&TaskNode, &TaskDependency<T, T::Output>)> + '_ {
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| d.as_task_dependency().map(|d| (n, d))))
  }
  /// Get all task nodes and corresponding dependencies that require file `dst`.
  #[inline]
  pub fn get_tasks_requiring_file<'a>(&'a self, dst: &'a FileNode) -> impl Iterator<Item=(&TaskNode, &FileDependency)> + '_ {
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| d.as_require_file_dependency().map(|d| (n, d))))
  }
  /// Get the task node that provides file `dst`, or `None` if there is none.
  #[inline]
  pub fn get_task_providing_file(&self, dst: &FileNode) -> Option<&TaskNode> {
    self.graph.get_incoming_edges(dst)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(n) } else { None })).next()
  }
  /// Get all task nodes and corresponding dependencies that require or provide file `dst`.
  #[inline]
  pub fn get_tasks_requiring_or_providing_file<'a>(&'a self, dst: &'a FileNode, provide: bool) -> impl Iterator<Item=(&TaskNode, &FileDependency)> + '_ {
    self.graph.get_incoming_edges(dst)
      .filter_map(move |(n, d)| d.as_ref().and_then(move |d| d.as_require_or_provide_file_dependency(provide).map(|d| (n, d))))
  }
  /// Get all file nodes that are provided by task `src`.
  #[inline]
  pub fn get_provided_files<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&FileNode> + '_ {
    self.graph.get_outgoing_edges(src)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(n) } else { None }))
  }
}
