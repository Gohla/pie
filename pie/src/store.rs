use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use pie_graph::{DAG, Node};

use crate::dependency::Dependency;
use crate::Task;

pub type TaskNode = Node;
pub type FileNode = Node;

#[derive(Serialize, Deserialize, Debug)]
pub(crate) struct Store<T: Task, H> {
  #[serde(bound(
  serialize = "T: Task + serde::Serialize, H: BuildHasher + Default, DAG<NodeData<T, T::Output>, ParentData, ChildData, H>: serde::Serialize",
  deserialize = "T: Task + serde::Deserialize<'de>, H: BuildHasher + Default, DAG<NodeData<T, T::Output>, ParentData, ChildData, H>: serde::Deserialize<'de>"
  ))] // Set bounds such that `H` does not have to be (de)serializable
  graph: DAG<NodeData<T, T::Output>, ParentData, ChildData, H>,
  #[serde(bound(
  serialize = "T: Task + serde::Serialize, H: BuildHasher + Default, HashMap<T, TaskNode, H>: serde::Serialize",
  deserialize = "T: Task + serde::Deserialize<'de>, H: BuildHasher + Default, HashMap<T, TaskNode, H>: serde::Deserialize<'de>"
  ))] // Set bounds such that `H` does not have to be (de)serializable
  task_to_node: HashMap<T, TaskNode, H>,
  #[serde(bound(
  serialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNode, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNode, H>: serde::Deserialize<'de>"
  ))] // Set bounds such that `H` does not have to be (de)serializable
  file_to_node: HashMap<PathBuf, FileNode, H>,
}

#[derive(Serialize, Deserialize, Debug)]
pub(crate) enum NodeData<T, O> {
  Task {
    task: T,
    dependencies: Option<Vec<Dependency<T, O>>>,
    output: Option<O>,
  },
  File(PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub(crate) enum ParentData {
  FileRequiringTask,
  FileProvidingTask,
  TaskRequiringTask,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, Debug)]
pub(crate) enum ChildData {
  RequireFile,
  ProvideFile,
  RequireTask,
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
        dependencies: None,
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
  pub fn add_task_dependency_edge(&mut self, depender_task_node: TaskNode, dependee_task_node: TaskNode) -> Result<bool, pie_graph::Error> {
    self.graph.add_dependency(depender_task_node, dependee_task_node, ParentData::TaskRequiringTask, ChildData::RequireTask)
  }


  #[inline]
  pub fn remove_dependencies_of_task(&mut self, task_node: &TaskNode) -> Option<Vec<Dependency<T, T::Output>>> {
    if let Some(NodeData::Task { dependencies, .. }) = self.graph.get_node_data_mut(task_node) {
      std::mem::take(dependencies)
    } else {
      None
    }
  }
  #[inline]
  pub fn set_dependencies_of_task(&mut self, task_node: TaskNode, new_dependencies: Vec<Dependency<T, T::Output>>) {
    if let Some(NodeData::Task { ref mut dependencies, .. }) = self.graph.get_node_data_mut(task_node) {
      std::mem::swap(dependencies, &mut Some(new_dependencies));
    }
  }
  #[inline]
  pub fn add_to_dependencies_of_task(&mut self, task_node: TaskNode, dependency: Dependency<T, T::Output>) {
    if let Some(NodeData::Task { ref mut dependencies, .. }) = self.graph.get_node_data_mut(task_node) {
      if let Some(dependencies) = dependencies {
        dependencies.push(dependency);
      } else {
        *dependencies = Some(vec![dependency]);
      }
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
    self.graph.add_dependency(depender_task_node, dependee_file_node, ParentData::FileRequiringTask, ChildData::RequireFile).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.add_to_dependencies_of_task(depender_task_node, dependency);
  }
  #[inline]
  pub fn add_file_provide_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: Dependency<T, T::Output>) {
    self.graph.add_dependency(depender_task_node, dependee_file_node, ParentData::FileProvidingTask, ChildData::ProvideFile).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.add_to_dependencies_of_task(depender_task_node, dependency);
  }


  #[inline]
  pub fn reset_task(&mut self, task_node: &TaskNode) {
    for dependee in self.graph.get_outgoing_dependency_nodes(task_node).cloned().collect::<Vec<_>>() { // OPTO: reuse allocation
      self.graph.remove_dependency(task_node, dependee);
    }
  }


  #[inline]
  pub fn get_providing_task_node(&self, file_node: &FileNode) -> Option<&TaskNode> {
    self.graph.get_incoming_dependencies(file_node).filter_map(|(n, pe)| if pe == &ParentData::FileProvidingTask { Some(n) } else { None }).next()
  }
  #[inline]
  pub fn get_requiring_task_nodes<'a>(&'a self, file_node: &'a FileNode) -> impl Iterator<Item=&TaskNode> + '_ {
    self.graph.get_incoming_dependencies(file_node).filter_map(|(n, pe)| if pe == &ParentData::FileRequiringTask { Some(n) } else { None })
  }
  #[inline]
  pub fn contains_transitive_task_dependency(&self, depender_task_node: &TaskNode, dependee_task_node: &TaskNode) -> bool {
    self.graph.contains_transitive_dependency(depender_task_node, dependee_task_node)
  }
}
