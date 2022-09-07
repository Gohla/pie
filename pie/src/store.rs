use std::collections::HashMap;
use std::hash::BuildHasher;
use std::path::PathBuf;

use pie_graph::{DAG, Node};

use crate::dependency::{Dependency, FileDependency};
use crate::prelude::DynOutput;
use crate::Task;
use crate::task::DynTask;

pub type TaskNode = Node;
pub type FileNode = Node;

#[derive(Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Store<C, H> {
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "H: BuildHasher + Default, DAG<NodeData<C>, ParentData, ChildData, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, DAG<NodeData<C>, ParentData, ChildData, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  graph: DAG<NodeData<C>, ParentData, ChildData, H>,
  #[cfg_attr(feature = "serde", serde(with = "task_to_node_serde"))]
  task_to_node: HashMap<Box<dyn DynTask>, TaskNode, H>,
  #[cfg_attr(feature = "serde", serde(bound(
  serialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNode, H>: serde::Serialize",
  deserialize = "H: BuildHasher + Default, HashMap<PathBuf, FileNode, H>: serde::Deserialize<'de>"
  )))] // Set bounds such that `H` does not have to be (de)serializable
  file_to_node: HashMap<PathBuf, FileNode, H>,
}

#[derive(Debug)]
pub enum NodeData<C> {
  Task {
    task: Box<dyn DynTask>,
    dependencies: Option<Vec<Box<dyn Dependency<C>>>>,
    output: Option<Box<dyn DynOutput>>,
  },
  File(PathBuf),
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ParentData {
  FileRequiringTask,
  FileProvidingTask,
  TaskRequiringTask,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum ChildData {
  RequireFile,
  ProvideFile,
  RequireTask,
}

impl<C, H: BuildHasher + Default> Default for Store<C, H> {
  #[inline]
  fn default() -> Self {
    Self {
      graph: DAG::with_default_hasher(),
      task_to_node: HashMap::default(),
      file_to_node: HashMap::default(),
    }
  }
}

impl<C, H: BuildHasher + Default> Store<C, H> {
  /// Creates a new `[Store]`.
  #[inline]
  pub fn new() -> Self { Default::default() }

  #[inline]
  pub fn get_or_create_node_by_task(&mut self, task: Box<dyn DynTask>) -> TaskNode {
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
  pub fn get_task_by_node(&self, task_node: &TaskNode) -> Option<&Box<dyn DynTask>> {
    self.graph.get_node_data(task_node).and_then(|d| match d {
      NodeData::Task { task, .. } => Some(task),
      _ => None
    })
  }

  #[inline]
  pub fn task_by_node(&self, task_node: &TaskNode) -> &Box<dyn DynTask> {
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
  pub fn remove_dependencies_of_task(&mut self, task_node: &TaskNode) -> Option<Vec<Box<dyn Dependency<C>>>> {
    if let Some(NodeData::Task { dependencies, .. }) = self.graph.get_node_data_mut(task_node) {
      std::mem::take(dependencies)
    } else {
      None
    }
  }
  #[inline]
  pub fn set_dependencies_of_task(&mut self, task_node: TaskNode, new_dependencies: Vec<Box<dyn Dependency<C>>>) {
    if let Some(NodeData::Task { ref mut dependencies, .. }) = self.graph.get_node_data_mut(task_node) {
      std::mem::swap(dependencies, &mut Some(new_dependencies));
    }
  }
  #[inline]
  pub fn add_to_dependencies_of_task(&mut self, task_node: TaskNode, dependency: Box<dyn Dependency<C>>) {
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
  pub fn get_task_output<T: Task>(&self, task_node: TaskNode) -> Option<&T::Output> {
    if let Some(NodeData::Task { output: Some(output), .. }) = self.graph.get_node_data(task_node) {
      // Note: `output.as_ref` is very important here, because `Box<dyn DynOutput>` also implements `DynOutput`, which 
      // in turn has an `as_any` method as well. However, `downcast_ref` will *always fail* on `Box<dyn DynOutput>` 
      // because it will try to downcast the box instead of what is inside the box.
      output.as_ref().as_any().downcast_ref::<T::Output>()
    } else {
      None
    }
  }
  #[inline]
  pub fn set_task_output<T: Task>(&mut self, task_node: TaskNode, new_output: T::Output) {
    if let Some(NodeData::Task { output, .. }) = self.graph.get_node_data_mut(task_node) {
      if let Some(output) = output {
        // Note: `output.as_mut` is very important here, for the same reason as commented in `get_task_output`.
        if let Some(output) = output.as_mut().as_any_mut().downcast_mut::<T::Output>() {
          *output = new_output; // Replace the value inside the box.
        } else { // Stored output is not of the correct type any more, replace it with a new boxed output.
          *output = Box::new(new_output)
        }
      } else { // No output was stored yet, create a new boxed output.
        *output = Some(Box::new(new_output))
      }
    }
  }


  #[inline]
  pub fn add_file_require_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: FileDependency) {
    self.graph.add_dependency(depender_task_node, dependee_file_node, ParentData::FileRequiringTask, ChildData::RequireFile).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.add_to_dependencies_of_task(depender_task_node, Box::new(dependency));
  }
  #[inline]
  pub fn add_file_provide_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: FileDependency) {
    self.graph.add_dependency(depender_task_node, dependee_file_node, ParentData::FileProvidingTask, ChildData::ProvideFile).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.add_to_dependencies_of_task(depender_task_node, Box::new(dependency));
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

// Custom serde implementations

#[cfg(feature = "serde")]
mod task_to_node_serde {
  use std::collections::HashMap;
  use std::hash::BuildHasher;

  use erased_serde::Serialize;
  use serde::{Deserializer, Serializer};
  use serde::ser::SerializeMap;

  use crate::task::DynTask;
  use crate::TaskNode;

  struct Wrap<'a, T: ?Sized>(&'a T);

  trait DowncastSerde<'a> {
    fn downcast_serialize(self) -> Option<&'a (dyn Serialize + 'a)>;
  }

  impl<'a, T: DynTask + ?Sized> DowncastSerde<'a> for Wrap<'a, T> {
    fn downcast_serialize(self) -> Option<&'a (dyn Serialize + 'a)> {
      None
    }
  }

  impl<'a, T: DynTask + serde::Serialize + ?Sized> DowncastSerde<'a> for &'a Wrap<'a, T> {
    fn downcast_serialize(self) -> Option<&'a (dyn Serialize + 'a)> {
      Some(&self.0 as &dyn Serialize)
    }
  }

  pub fn serialize<H, S>(task_to_node: &HashMap<Box<dyn DynTask>, TaskNode, H>, serializer: S) -> Result<S::Ok, S::Error> where
    H: BuildHasher + Default,
    S: Serializer,
  {
    let mut map = serializer.serialize_map(None)?;
    for (k, v) in task_to_node {
      let k = Wrap(k.as_ref());
      if let Some(k) = k.downcast_serialize() {
        map.serialize_entry(k, v)?;
      }
    }
    map.end()
  }

  pub fn deserialize<'de, H, D>(_deserializer: D) -> Result<HashMap<Box<dyn DynTask>, TaskNode, H>, D::Error> where
    H: BuildHasher + Default,
    D: Deserializer<'de>,
  {
    todo!()
  }
}

#[cfg(feature = "serde")]
impl<C> serde::Serialize for NodeData<C> {
  fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
    todo!()
  }
}

#[cfg(feature = "serde")]
impl<'de, C> serde::Deserialize<'de> for NodeData<C> {
  fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
    todo!()
  }
}
