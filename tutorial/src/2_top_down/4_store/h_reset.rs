impl<T: Task> Store<T, T::Output> {
  /// Reset task `src`, removing its output and removing all its dependencies.
  pub fn reset_task(&mut self, src: &TaskNode) {
    if let Some(data) = self.graph.get_node_data_mut(src) {
      match data {
        NodeData::Task { output, .. } => *output = None,
        _ => {}
      }
    }
    self.graph.remove_edges_of_node(src);
  }
}
