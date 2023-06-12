impl<T: Task> Store<T, T::Output> {
  /// Get all dependencies of task `src`.
  pub fn get_dependencies_of_task<'a>(&'a self, src: &'a TaskNode) -> impl Iterator<Item=&'a Option<Dependency<T, T::Output>>> + 'a {
    self.graph.get_outgoing_edge_data(src)
  }
  /// Add a file require dependency from task `src` to file `dst`.
  pub fn add_file_require_dependency(&mut self, src: &TaskNode, dst: &FileNode, dependency: FileDependency) {
    // Ignore Result: cycles cannot occur from task to file dependencies, as files do not have dependencies.
    let _ = self.graph.add_edge(src, dst, Some(Dependency::RequireFile(dependency)));
  }
  /// Reserve a task require dependency from task `src` to task `dst`. Returns an `Err` if this dependency creates a 
  /// cycle. This reservation is required because the dependency from `src` to `dst` should already exist for 
  /// cycle checking, but we do not yet have the output of task `dst` so we cannot fully create the dependency.
  pub fn reserve_task_require_dependency(&mut self, src: &TaskNode, dst: &Node) -> Result<(), pie_graph::Error> {
    self.graph.add_edge(src, dst, None)?;
    Ok(())
  }
  /// Update the reserved task require dependency from task `src` to `dst` to `dependency`. Panics if the dependency was
  /// not reserved before.
  pub fn update_reserved_task_require_dependency(&mut self, src: &TaskNode, dst: &Node, dependency: TaskDependency<T, T::Output>) {
    self.graph.get_edge_data_mut(src, dst).unwrap().replace(Dependency::RequireTask(dependency));
  }
}
