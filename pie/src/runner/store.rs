use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use anymap::AnyMap;
use bimap::BiHashMap;
use incremental_topo::{IncrementalTopo, Node};

use crate::{Context, Dependency, DynTask, FileDependency, Task};

pub type TaskNode = Node;
pub type FileNode = Node;

pub struct Store<C: Context> {
  graph: IncrementalTopo,
  task_node: BiHashMap<Box<dyn DynTask>, TaskNode>,
  file_node: BiHashMap<PathBuf, FileNode>,

  task_to_required_files: HashMap<TaskNode, HashSet<FileNode>>,
  file_to_requiring_tasks: HashMap<FileNode, HashSet<TaskNode>>,
  task_to_provided_files: HashMap<TaskNode, HashSet<TaskNode>>,
  file_to_providing_task: HashMap<FileNode, TaskNode>,
  task_to_required_tasks: HashMap<TaskNode, HashSet<TaskNode>>,

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
      task_to_provided_files: HashMap::new(),
      file_to_providing_task: HashMap::new(),
      task_to_required_tasks: HashMap::default(),

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
  pub fn get_task_by_node(&self, task_node: &TaskNode) -> Option<&Box<dyn DynTask>> {
    self.task_node.get_by_right(task_node)
  }

  #[inline]
  pub fn task_by_node(&self, task_node: &TaskNode) -> &Box<dyn DynTask> {
    self.task_node.get_by_right(task_node).unwrap()
  }


  #[inline]
  pub fn get_or_create_file_node(&mut self, path: &PathBuf) -> FileNode {
    // TODO: normalize path?
    if let Some(file_node) = self.file_node.get_by_left(path) {
      *file_node
    } else {
      let node = self.graph.add_node();
      self.file_node.insert(path.clone(), node);
      node
    }
  }


  #[inline]
  pub fn add_task_dependency_edge(&mut self, depender_task_node: TaskNode, dependee_task_node: TaskNode) -> Result<bool, incremental_topo::Error> {
    let result = self.graph.add_dependency(depender_task_node, dependee_task_node)?;
    self.task_to_required_tasks.entry(depender_task_node).or_insert_with(|| HashSet::with_capacity(1)).insert(dependee_task_node);
    Ok(result)
  }


  #[inline]
  pub fn remove_dependencies_of_task(&mut self, task_node: &TaskNode) -> Option<Vec<Box<dyn Dependency<C>>>> {
    self.task_dependencies.remove(task_node)
  }
  #[inline]
  pub fn set_dependencies_of_task(&mut self, task_node: TaskNode, dependencies: Vec<Box<dyn Dependency<C>>>) {
    self.task_dependencies.insert(task_node, dependencies);
  }
  #[inline]
  pub fn add_to_dependencies_of_task(&mut self, task_node: TaskNode, dependency: Box<dyn Dependency<C>>) {
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


  #[inline]
  pub fn add_file_require_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: FileDependency) {
    self.graph.add_dependency(depender_task_node, dependee_file_node).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.task_to_required_files.entry(depender_task_node).or_insert_with(|| HashSet::with_capacity(1)).insert(dependee_file_node);
    self.file_to_requiring_tasks.entry(dependee_file_node).or_insert_with(|| HashSet::with_capacity(1)).insert(depender_task_node);
    self.add_to_dependencies_of_task(depender_task_node, Box::new(dependency));
  }
  #[inline]
  pub fn add_file_provide_dependency(&mut self, depender_task_node: TaskNode, dependee_file_node: FileNode, dependency: FileDependency) {
    self.graph.add_dependency(depender_task_node, dependee_file_node).ok(); // Ignore error OK: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    self.task_to_provided_files.entry(depender_task_node).or_insert_with(|| HashSet::with_capacity(1)).insert(dependee_file_node);
    self.file_to_providing_task.insert(dependee_file_node, depender_task_node);
    self.add_to_dependencies_of_task(depender_task_node, Box::new(dependency));
  }

  #[inline]
  pub fn reset_task(&mut self, task_node: &TaskNode) {
    // Remove required files of task.
    if let Some(required_files) = self.task_to_required_files.remove(task_node) {
      for required_file in required_files {
        self.graph.delete_dependency(task_node, required_file);
        if let Some(requiring_tasks) = self.file_to_requiring_tasks.get_mut(&required_file) {
          requiring_tasks.remove(task_node);
        }
      }
    }
    // Remove provided files of task.
    if let Some(provided_files) = self.task_to_provided_files.remove(task_node) {
      for provided_file in provided_files {
        self.graph.delete_dependency(task_node, provided_file);
        self.file_to_providing_task.remove(&provided_file);
      }
    }
    // Remove required tasks of task.
    if let Some(required_tasks) = self.task_to_required_tasks.remove(task_node) {
      for required_task in required_tasks {
        self.graph.delete_dependency(task_node, required_task);
      }
    }
  }

  #[inline]
  pub fn get_providing_task_node(&self, file_node: &FileNode) -> Option<&TaskNode> {
    self.file_to_providing_task.get(file_node)
  }

  #[inline]
  pub fn get_requiring_task_node(&self, file_node: &FileNode) -> Option<&HashSet<TaskNode>> {
    self.file_to_requiring_tasks.get(file_node)
  }

  #[inline]
  pub fn contains_transitive_task_dependency(&self, depender_task_node: &TaskNode, dependee_task_node: &TaskNode) -> bool {
    self.graph.contains_transitive_dependency(depender_task_node, dependee_task_node)
  }
}
