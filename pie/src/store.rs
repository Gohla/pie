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
  pub graph: DAG<NodeData<T, O>, Option<Dependency<T, O>>, H>,
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
  #[inline]
  pub fn get_or_create_node_by_task(&mut self, task: &T) -> TaskNode {
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
  #[inline]
  pub fn get_task_by_node(&self, node: &TaskNode) -> Option<&T> {
    self.graph.get_node_data(node).and_then(|d| match d {
      NodeData::Task { task, .. } => Some(task),
      _ => None
    })
  }
  #[inline]
  pub fn task_by_node(&self, node: &TaskNode) -> &T {
    self.get_task_by_node(node).unwrap()
  }


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
  #[inline]
  pub fn get_path_by_node(&self, node: &FileNode) -> Option<&PathBuf> {
    self.graph.get_node_data(node).and_then(|d| match d {
      NodeData::File(path) => Some(path),
      _ => None
    })
  }
  #[inline]
  pub fn path_by_node(&self, node: &FileNode) -> &PathBuf {
    self.get_path_by_node(node).unwrap()
  }

  #[inline]
  pub fn get_dependencies_of_task<'a>(&'a self, depender: &'a TaskNode) -> impl Iterator<Item=Node> + 'a {
    self.graph.get_outgoing_edge_nodes(depender).copied()
  }
  #[inline]
  pub fn add_to_dependencies_of_task(&mut self, depender: &TaskNode, dependee: &Node, dependency: Option<Dependency<T, T::Output>>) -> Result<(), pie_graph::Error> {
    self.graph.add_edge(depender, dependee, dependency)?;
    Ok(())
  }
  #[inline]
  pub fn update_dependency_of_task(&mut self, depender: &TaskNode, dependee: &Node, dependency: Option<Dependency<T, T::Output>>) {
    if let Some(data) = self.graph.get_edge_data_mut(depender, dependee) {
      *data = dependency;
    }
  }


  #[inline]
  pub fn task_has_output(&self, node: &TaskNode) -> bool {
    if let Some(NodeData::Task { output: Some(_), .. }) = self.graph.get_node_data(node) {
      true
    } else {
      false
    }
  }
  #[inline]
  pub fn get_task_output(&self, node: &TaskNode) -> Option<&T::Output> {
    if let Some(NodeData::Task { output: Some(output), .. }) = self.graph.get_node_data(node) {
      Some(output)
    } else {
      None
    }
  }
  #[inline]
  pub fn set_task_output(&mut self, node: &TaskNode, new_output: T::Output) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(node) {
      if let Some(output) = output { // Replace the output.
        *output = new_output;
      } else { // No output was stored yet, create a new boxed output.
        *output = Some(new_output);
      }
    }
  }


  #[inline]
  pub fn add_file_require_dependency(&mut self, depender: &TaskNode, dependee: &FileNode, dependency: FileDependency) {
    self.graph.add_edge(depender, dependee, Some(Dependency::RequireFile(dependency))).ok(); // OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
  }
  #[inline]
  pub fn add_file_provide_dependency(&mut self, depender: &TaskNode, dependee: &FileNode, dependency: FileDependency) {
    self.graph.add_edge(depender, dependee, Some(Dependency::ProvideFile(dependency))).ok(); // OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
  }


  #[inline]
  pub fn reset_task(&mut self, task_node: &TaskNode) {
    self.graph.remove_edges_of_node(task_node);
    // TODO: should this remove output?
  }


  /// Checks whether there is a direct or transitive dependency from `depender_task_node` to `dependee_task_node`.
  #[inline]
  pub fn contains_transitive_task_dependency(&self, depender: &TaskNode, dependee: &TaskNode) -> bool {
    self.graph.contains_transitive_edge(depender, dependee)
  }


  /// Get all requirer task nodes and corresponding dependencies of tasks that require given `task_node`.
  #[inline]
  pub fn get_tasks_requiring_task<'a>(&'a self, node: &'a TaskNode) -> impl Iterator<Item=(&TaskNode, &TaskDependency<T, T::Output>)> + '_ {
    self.graph.get_incoming_edges(node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| d.as_task_dependency().map(|d| (n, d))))
  }
  /// Get all requirer task nodes and corresponding dependencies of tasks that require given `file_node`.
  #[inline]
  pub fn get_tasks_requiring_file<'a>(&'a self, node: &'a FileNode) -> impl Iterator<Item=(&TaskNode, &FileDependency)> + '_ {
    self.graph.get_incoming_edges(node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| d.as_require_file_dependency().map(|d| (n, d))))
  }
  /// Get the node of the tasks that provide given `file_node`, or `None` if there is none.
  #[inline]
  pub fn get_task_providing_file(&self, node: &FileNode) -> Option<&TaskNode> {
    self.graph.get_incoming_edges(node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(n) } else { None })).next()
  }
  /// Get all requirer task nodes and corresponding dependencies of tasks that require or provide given `file_node`.
  #[inline]
  pub fn get_tasks_requiring_or_providing_file<'a>(&'a self, node: &'a FileNode, provide: bool) -> impl Iterator<Item=(&TaskNode, &FileDependency)> + '_ {
    self.graph.get_incoming_edges(node)
      .filter_map(move |(n, d)| d.as_ref().and_then(move |d| d.as_require_or_provide_file_dependency(provide).map(|d| (n, d))))
  }
  /// Get all file nodes of files that are provided by given `task_node`.
  #[inline]
  pub fn get_provided_files<'a>(&'a self, node: &'a TaskNode) -> impl Iterator<Item=&FileNode> + '_ {
    self.graph.get_outgoing_edges(node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(n) } else { None }))
  }
}
