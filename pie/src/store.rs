use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::PathBuf;

use pie_graph::{DAG, NodeId};

use crate::dependency::Dependency;
use crate::Task;

pub type TaskNodeId = NodeId;
pub type FileNodeId = NodeId;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub(crate) struct Store<T: Task, H> {
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "T: Task + serde::Serialize, H: BuildHasher + Default, DAG<NodeData<T, T::Output>, Option<Dependency<T, T::Output>>, H>: serde::Serialize",
  deserialize = "T: Task + serde::Deserialize<'de>, H: BuildHasher + Default, DAG<NodeData<T, T::Output>, Option<Dependency<T, T::Output>>, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  pub graph: DAG<NodeData<T, T::Output>, Option<Dependency<T, T::Output>>, H>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "T: Task + serde::Serialize, H: BuildHasher + Default, HashMap<T, TaskNodeId, H>: serde::Serialize",
  deserialize = "T: Task + serde::Deserialize<'de>, H: BuildHasher + Default, HashMap<T, TaskNodeId, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  task_to_node: HashMap<T, TaskNodeId, H>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNodeId, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNodeId, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  file_to_node: HashMap<PathBuf, FileNodeId, H>,
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
  pub fn get_or_create_node_by_task(&mut self, task: T) -> TaskNodeId {
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
  pub fn get_task_by_node(&self, task_node: &TaskNodeId) -> Option<&T> {
    self.graph.get_node_data(task_node).and_then(|d| match d {
      NodeData::Task { task, .. } => Some(task),
      _ => None
    })
  }

  #[inline]
  pub fn task_by_node(&self, task_node: &TaskNodeId) -> &T {
    self.get_task_by_node(task_node).unwrap()
  }


  #[inline]
  pub fn get_or_create_file_node(&mut self, path: &PathBuf) -> FileNodeId {
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
  pub fn remove_dependencies_of_task(&mut self, depender: &TaskNodeId) -> Option<Vec<(NodeId, Option<Dependency<T, T::Output>>)>> {
    self.graph.remove_dependencies_of_node(depender)
  }
  #[inline]
  pub fn set_dependencies_of_task(&mut self, depender: &TaskNodeId, new_dependencies: Vec<(NodeId, Option<Dependency<T, T::Output>>)>) -> Result<(), pie_graph::Error> {
    self.graph.remove_dependencies_of_node(depender);
    for (dependee, dependency) in new_dependencies {
      self.graph.add_edge(depender, dependee, dependency)?;
    }
    Ok(())
  }
  #[inline]
  pub fn add_to_dependencies_of_task(&mut self, depender: &TaskNodeId, dependee: &NodeId, dependency: Option<Dependency<T, T::Output>>) -> Result<(), pie_graph::Error> {
    self.graph.add_edge(depender, dependee, dependency)?;
    Ok(())
  }
  #[inline]
  pub fn update_dependency_of_task(&mut self, depender: &TaskNodeId, dependee: &NodeId, dependency: Option<Dependency<T, T::Output>>) {
    if let Some(data) = self.graph.get_dependency_data_mut(depender, dependee) {
      *data = dependency;
    }
  }


  #[inline]
  pub fn task_has_output(&self, task_node: &TaskNodeId) -> bool {
    if let Some(NodeData::Task { output: Some(_), .. }) = self.graph.get_node_data(task_node) {
      true
    } else {
      false
    }
  }
  #[inline]
  pub fn get_task_output(&self, task_node: &TaskNodeId) -> Option<&T::Output> {
    if let Some(NodeData::Task { output: Some(output), .. }) = self.graph.get_node_data(task_node) {
      Some(output)
    } else {
      None
    }
  }
  #[inline]
  pub fn set_task_output(&mut self, task_node: &TaskNodeId, new_output: T::Output) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(task_node) {
      if let Some(output) = output { // Replace the output.
        *output = new_output;
      } else { // No output was stored yet, create a new boxed output.
        *output = Some(new_output);
      }
    }
  }


  #[inline]
  pub fn add_file_require_dependency(&mut self, depender: &TaskNodeId, dependee: &FileNodeId, dependency: Dependency<T, T::Output>) {
    self.graph.add_edge(depender, dependee, Some(dependency)).ok(); // OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
  }
  #[inline]
  pub fn add_file_provide_dependency(&mut self, depender: &TaskNodeId, dependee: &FileNodeId, dependency: Dependency<T, T::Output>) {
    self.graph.add_edge(depender, dependee, Some(dependency)).ok(); // OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
  }


  #[inline]
  pub fn reset_task(&mut self, task_node: &TaskNodeId) {
    self.graph.remove_dependencies_of_node(task_node);
    // TODO: should this remove output?
  }


  /// Checks whether there is a direct or transitive dependency from `depender_task_node` to `dependee_task_node`.
  #[inline]
  pub fn contains_transitive_task_dependency(&self, depender: &TaskNodeId, dependee: &TaskNodeId) -> bool {
    self.graph.contains_transitive_dependency(depender, dependee)
  }


  /// Get all requirer task nodes and corresponding dependencies of tasks that require given `task_node`.
  #[inline]
  pub fn get_tasks_requiring_task<'a>(&'a self, task_node: &'a TaskNodeId) -> impl Iterator<Item=(&TaskNodeId, &Dependency<T, T::Output>)> + '_ {
    self.graph.get_incoming_dependencies(task_node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_require_task() { Some((n, d)) } else { None }))
  }
  /// Get all requirer task nodes and corresponding dependencies of tasks that require given `file_node`.
  #[inline]
  pub fn get_tasks_requiring_file<'a>(&'a self, file_node: &'a FileNodeId) -> impl Iterator<Item=(&TaskNodeId, &Dependency<T, T::Output>)> + '_ {
    self.graph.get_incoming_dependencies(file_node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_require_file() { Some((n, d)) } else { None }))
  }
  /// Get the node of the tasks that provide given `file_node`, or `None` if there is none.
  #[inline]
  pub fn get_task_providing_file(&self, file_node: &FileNodeId) -> Option<&TaskNodeId> {
    self.graph.get_incoming_dependencies(file_node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(n) } else { None })).next()
  }
  /// Get all requirer task nodes and corresponding dependencies of tasks that require or provide given `file_node`.
  #[inline]
  pub fn get_tasks_requiring_or_providing_file<'a>(&'a self, file_node: &'a FileNodeId) -> impl Iterator<Item=(&TaskNodeId, &Dependency<T, T::Output>)> + '_ {
    self.graph.get_incoming_dependencies(file_node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_file_dependency() { Some((n, d)) } else { None }))
  }
  /// Get all file nodes of files that are provided by given `task_node`.
  #[inline]
  pub fn get_provided_files<'a>(&'a self, task_node: &'a TaskNodeId) -> impl Iterator<Item=&FileNodeId> + '_ {
    self.graph.get_outgoing_dependencies(task_node)
      .filter_map(|(n, d)| d.as_ref().and_then(|d| if d.is_provide_file() { Some(n) } else { None }))
  }
}
