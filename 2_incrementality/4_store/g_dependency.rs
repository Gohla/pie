
impl<T: Task> Store<T, T::Output> {
  /// Get all dependencies of task `src`.
  ///
  /// # Panics
  ///
  /// Panics in development builds if `src` was not found in the dependency graph.
  pub fn get_dependencies_of_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Dependency<T, T::Output>> + 'a {
    debug_assert!(self.graph.contains_node(src), "BUG: node {:?} was not found in the dependency graph", src);
    self.graph.get_outgoing_edge_data(src)
  }
  /// Add a file require `dependency` from task `src` to file `dst`.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph, or if a cycle is created by adding this dependency.
  pub fn add_file_require_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    match self.graph.add_edge(src, dst, Dependency::RequireFile(dependency)) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => panic!("BUG: cycle detected when adding file dependency from {:?} to {:?}", src, dst),
      _ => {},
    }
  }
  /// Adds a task require `dependency` from task `src` to task `dst`.
  ///
  /// # Errors
  ///
  /// Returns `Err(())` if adding this dependency to the graph creates a cycle.
  ///
  /// # Panics
  ///
  /// Panics if `src` or `dst` were not found in the dependency graph.
  pub fn add_task_require_dependency(&mut self, src: &TaskNode, dst: &TaskNode, dependency: TaskDependency<T, T::Output>) -> Result<(), ()> {
    match self.graph.add_edge(src, dst, Dependency::RequireTask(dependency)) {
      Err(pie_graph::Error::NodeMissing) => panic!("BUG: source node {:?} or destination node {:?} was not found in the dependency graph", src, dst),
      Err(pie_graph::Error::CycleDetected) => Err(()),
      _ => Ok(()),
    }
  }
}
