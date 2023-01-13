use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::PathBuf;

use pie_graph::{DAG, NodeId};

use crate::dependency::Dependency;
use crate::Task;

pub type TaskNode = NodeId;
pub type FileNode = NodeId;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct Store<T: Task, H> {
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "T: Task + serde::Serialize, H: BuildHasher + Default, DAG<NodeData<T, T::Output>, Dependency<T, T::Output>, H>: serde::Serialize",
  deserialize = "T: Task + serde::Deserialize<'de>, H: BuildHasher + Default, DAG<NodeData<T, T::Output>, Dependency<T, T::Output>, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  pub graph: DAG<NodeData<T, T::Output>, Dependency<T, T::Output>, H>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "T: Task + serde::Serialize, H: BuildHasher + Default, HashMap<T, TaskNode, H>: serde::Serialize",
  deserialize = "T: Task + serde::Deserialize<'de>, H: BuildHasher + Default, HashMap<T, TaskNode, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  task_to_node: HashMap<T, TaskNode, H>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNode, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNode, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  file_to_node: HashMap<PathBuf, FileNode, H>,
}

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) enum NodeData<T, O> {
  Task {
    task: T,
    output: Option<O>,
  },
  File(PathBuf),
}

impl<T: Task, H: BuildHasher + Default> Default for Store<T, H> {
  #[inline]
  fn default() -> Self {
    Self {
      graph: DAG::with_default_hasher(),
      task_to_node: HashMap::default(),
      file_to_node: HashMap::default(),
    }
  }
}

impl<T: Task, H: BuildHasher + Default> Store<T, H> {
  #[inline]
  pub fn get_or_create_node_by_task(&mut self, task: T) -> TaskNode {
    if let Some(node) = self.task_to_node.get(&task) {
      *node
    } else {
      let node = self.graph.add_node(NodeData::Task {
        task: task.clone(),
        output: None,
      });
      self.task_to_node.insert(task, node);
      node
    }
  }
  #[inline]
  pub fn get_task_by_node(&self, task_node: &TaskNode) -> Option<&T> {
    self.graph.get_node_data(task_node).and_then(|d| match d {
      NodeData::Task { task, .. } => Some(task),
      _ => None
    })
  }

  #[inline]
  pub fn task_by_node(&self, task_node: &TaskNode) -> &T {
    self.get_task_by_node(task_node).unwrap()
  }


  #[inline]
  pub fn get_or_create_file_node(&mut self, path: &PathBuf) -> FileNode {
    // TODO: normalize path?
    if let Some(file_node) = self.file_to_node.get(path) {
      *file_node
    } else {
      let node = self.graph.add_node(NodeData::File(path.clone()));
      self.file_to_node.insert(path.clone(), node);
      node
    }
  }


  #[inline]
  pub fn remove_dependencies_of_task(&mut self, task_node: &TaskNode) -> Option<Vec<Dependency<T, T::Output>>> {
    self.graph.remove_dependencies_of_node(task_node)
  }
  #[inline]
  pub fn set_dependencies_of_task(&mut self, task_node: TaskNode, new_dependencies: Vec<Dependency<T, T::Output>>) -> Result<(), pie_graph::Error> {
    self.graph.remove_dependencies_of_node(task_node);
    for dependency in new_dependencies {
      let target = self.get_dependency_target(&dependency);
      self.graph.add_edge(task_node, target, dependency)?;
    }
    Ok(())
  }
  #[inline]
  pub fn would_dependency_edge_induce_cycle(&mut self, depender: TaskNode, dependee: TaskNode) -> bool {
    self.graph.would_edge_induce_cycle(depender, dependee)
  }
  #[inline]
  pub fn add_to_dependencies_of_task(&mut self, task_node: TaskNode, dependency: Dependency<T, T::Output>) -> Result<(), pie_graph::Error> {
    let target = self.get_dependency_target(&dependency);
    self.graph.add_edge(task_node, target, dependency)?;
    Ok(())
  }
  #[inline]
  fn get_dependency_target(&self, dependency: &Dependency<T, T::Output>) -> NodeId {
    match dependency {
      Dependency::RequireFile(path, _, _) => self.file_to_node[path],
      Dependency::ProvideFile(path, _, _) => self.file_to_node[path],
      Dependency::RequireTask(task, _, _) => self.task_to_node[task],
    }
  }


  #[inline]
  pub fn task_has_output(&self, task_node: TaskNode) -> bool {
    if let Some(NodeData::Task { output: Some(_), .. }) = self.graph.get_node_data(task_node) {
      true
    } else {
      false
    }
  }
  #[inline]
  pub fn get_task_output(&self, task_node: TaskNode) -> Option<&T::Output> {
    if let Some(NodeData::Task { output: Some(output), .. }) = self.graph.get_node_data(task_node) {
      Some(output)
    } else {
      None
    }
  }
  #[inline]
  pub fn set_task_output(&mut self, task_node: TaskNode, new_output: T::Output) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(task_node) {
      if let Some(output) = output { // Replace the output.
        *output = new_output;
      } else { // No output was stored yet, create a new boxed output.
        *output = Some(new_output);
      }
    }
  }


  #[inline]
  pub fn add_file_require_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: Dependency<T, T::Output>) {
    self.graph.add_edge(depender_task_node, dependee_file_node, dependency).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
  }
  #[inline]
  pub fn add_file_provide_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: Dependency<T, T::Output>) {
    self.graph.add_edge(depender_task_node, dependee_file_node, dependency).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
  }


  #[inline]
  pub fn reset_task(&mut self, task_node: &TaskNode) {
    self.graph.remove_dependencies_of_node(task_node);
    // TODO: should this remove output?
  }

  /// Get all task nodes that require given `task_node`.
  #[inline]
  pub fn get_task_nodes_requiring_task<'a>(&'a self, task_node: &'a TaskNode) -> impl Iterator<Item=&TaskNode> + '_ {
    self.graph.get_incoming_dependencies(task_node).filter_map(|(n, d)| if d.is_task_require() { Some(n) } else { None })
  }
  /// Checks whether there is a direct or transitive dependency from `depender_task_node` to `dependee_task_node`.
  #[inline]
  pub fn contains_transitive_task_dependency(&self, depender_task_node: &TaskNode, dependee_task_node: &TaskNode) -> bool {
    self.graph.contains_transitive_dependency(depender_task_node, dependee_task_node)
  }

  /// Get all task nodes that require given `file_node`.
  #[inline]
  pub fn get_task_nodes_requiring_file<'a>(&'a self, file_node: &'a FileNode) -> impl Iterator<Item=&TaskNode> + '_ {
    self.graph.get_incoming_dependencies(file_node).filter_map(|(n, d)| if d.is_require_file() { Some(n) } else { None })
  }
  /// Get the task node that provides given `file_node`, or `None` if there is none.
  #[inline]
  pub fn get_task_node_providing_file(&self, file_node: &FileNode) -> Option<&TaskNode> {
    self.graph.get_incoming_dependencies(file_node).filter_map(|(n, d)| if d.is_provide_file() { Some(n) } else { None }).next()
  }
}
