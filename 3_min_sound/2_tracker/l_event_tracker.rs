
impl<T: Task> Tracker<T> for EventTracker<T, T::Output> {
  fn build_start(&mut self) {
    self.events.clear();
  }

  fn require_file_end(&mut self, dependency: &FileDependency) {
    let data = RequireFileEnd {
      path: dependency.path().into(),
      stamper: *dependency.stamper(),
      stamp: *dependency.stamp(),
      index: self.events.len()
    };
    self.events.push(Event::RequireFileEnd(data));
  }
  fn require_task_start(&mut self, task: &T, stamper: &OutputStamper) {
    let data = RequireTaskStart { task: task.clone(), stamper: stamper.clone(), index: self.events.len() };
    self.events.push(Event::RequireTaskStart(data));
  }
  fn require_task_end(&mut self, dependency: &TaskDependency<T, T::Output>, output: &T::Output, was_executed: bool) {
    let data = RequireTaskEnd {
      task: dependency.task().clone(),
      stamper: *dependency.stamper(),
      stamp: dependency.stamp().clone(),
      output: output.clone(),
      was_executed,
      index: self.events.len()
    };
    self.events.push(Event::RequireTaskEnd(data));
  }

  fn execute_start(&mut self, task: &T) {
    let data = ExecuteStart { task: task.clone(), index: self.events.len() };
    self.events.push(Event::ExecuteStart(data));
  }
  fn execute_end(&mut self, task: &T, output: &T::Output) {
    let data = ExecuteEnd { task: task.clone(), output: output.clone(), index: self.events.len() };
    self.events.push(Event::ExecuteEnd(data));
  }
}
