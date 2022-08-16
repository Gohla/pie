use std::collections::HashMap;
use std::path::PathBuf;

use anymap::AnyMap;
use bimap::BiHashMap;
use incremental_topo::{IncrementalTopo, Node};

use crate::{Context, Dependency, DynTask, Task};

pub type TaskNode = Node;
pub type FileNode = Node;

pub struct Store<C: Context> {
  graph: IncrementalTopo,
  task_node: BiHashMap<Box<dyn DynTask>, TaskNode>,
  file_node: BiHashMap<PathBuf, FileNode>,

  task_to_required_files: HashMap<TaskNode, Vec<Node>>,
  file_to_requiring_tasks: HashMap<FileNode, Vec<TaskNode>>,
  task_to_provided_file: HashMap<TaskNode, Node>,
  file_to_providing_task: HashMap<FileNode, TaskNode>,

  task_outputs: AnyMap,
  task_dependencies: HashMap<TaskNode, Vec<Box<dyn Dependency<C>>>>,
}

impl<C: Context> Store<C> {
  /// Creates a new `[Store]`.
  pub fn new() -> Self {
    Self {
      graph: IncrementalTopo::new(),
      task_node: BiHashMap::new(),
      file_node: BiHashMap::new(),

      task_to_required_files: HashMap::new(),
      file_to_requiring_tasks: HashMap::new(),
      task_to_provided_file: HashMap::new(),
      file_to_providing_task: HashMap::new(),

      task_outputs: AnyMap::new(),
      task_dependencies: HashMap::new(),
    }
  }
}

impl<C: Context> Store<C> {
  #[inline]
  pub fn get_or_create_node_by_task(&mut self, task: Box<dyn DynTask>) -> TaskNode {
    if let Some(node) = self.task_node.get_by_left(&task) {
      *node
    } else {
      let node = self.graph.add_node();
      self.task_node.insert(task, node);
      node
    }
  }
  #[inline]
  pub fn get_task_by_node(&mut self, task_node: &TaskNode) -> Option<&Box<dyn DynTask>> {
    self.task_node.get_by_right(task_node)
  }


  #[inline]
  pub fn get_or_create_file_node(&mut self, path: &PathBuf) -> FileNode {
    // TODO: normalize path
    if let Some(file_node) = self.file_node.get_by_left(path) {
      *file_node
    } else {
      let node = self.graph.add_node();
      self.file_node.insert(path.clone(), node);
      node
    }
  }

  #[inline]
  pub fn remove_task_dependencies(&mut self, task_node: &TaskNode) -> Option<Vec<Box<dyn Dependency<C>>>> {
    self.task_dependencies.remove(task_node)
  }
  #[inline]
  pub fn set_task_dependencies(&mut self, task_node: TaskNode, dependencies: Vec<Box<dyn Dependency<C>>>) {
    self.task_dependencies.insert(task_node, dependencies);
  }
  #[inline]
  pub fn add_to_task_dependencies(&mut self, task_node: TaskNode, dependency: Box<dyn Dependency<C>>) {
    let dependencies = self.task_dependencies.entry(task_node).or_insert_with(|| Vec::new());
    dependencies.push(dependency);
  }

  #[inline]
  pub fn get_task_output_map<T: Task>(&self) -> Option<&HashMap<T, T::Output>> {
    self.task_outputs.get::<HashMap<T, T::Output>>()
  }
  #[inline]
  pub fn get_task_output_map_mut<T: Task>(&mut self) -> &mut HashMap<T, T::Output> {
    self.task_outputs.entry::<HashMap<T, T::Output>>().or_insert_with(|| HashMap::default())
  }
  #[inline]
  pub fn get_task_output<T: Task>(&self, task: &T) -> Option<&T::Output> {
    self.get_task_output_map::<T>().map_or(None, |map| map.get(task))
  }
  #[inline]
  pub fn set_task_output<T: Task>(&mut self, task: T, output: T::Output) {
    self.get_task_output_map_mut::<T>().insert(task, output);
  }
}
