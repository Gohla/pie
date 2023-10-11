
impl<T: Task> EventTracker<T, T::Output> {
  /// Returns a slice over all events.
  pub fn slice(&self) -> &[Event<T, T::Output>] {
    &self.events
  }
  /// Returns an iterator over all events.
  pub fn iter(&self) -> impl Iterator<Item=&Event<T, T::Output>> {
    self.events.iter()
  }

  /// Returns `true` if `predicate` returns `true` for any event.
  pub fn any(&self, predicate: impl FnMut(&Event<T, T::Output>) -> bool) -> bool {
    self.iter().any(predicate)
  }
  /// Returns `true` if `predicate` returns `true` for exactly one event.
  pub fn one(&self, predicate: impl FnMut(&&Event<T, T::Output>) -> bool) -> bool {
    self.iter().filter(predicate).count() == 1
  }

  /// Returns `Some(v)` for the first event `e` where `f(e)` returns `Some(v)`, or `None` otherwise.
  pub fn find_map<R>(&self, f: impl FnMut(&Event<T, T::Output>) -> Option<&R>) -> Option<&R> {
    self.iter().find_map(f)
  }


  /// Finds the first [require file end event](Event::RequireFileEnd) for `path` and returns `Some(&data)`, or `None`
  /// otherwise.
  pub fn first_require_file(&self, path: &PathBuf) -> Option<&RequireFileEnd> {
    self.find_map(|e| e.match_require_file_end(path))
  }
  /// Finds the first [require file end event](Event::RequireFileEnd) for `path` and returns `Some(&index)`, or `None`
  /// otherwise.
  pub fn first_require_file_index(&self, path: &PathBuf) -> Option<&usize> {
    self.first_require_file(path).map(|d| &d.index)
  }

  /// Finds the first require [start](Event::RequireTaskStart) and [end](Event::RequireTaskEnd) event for `task` and
  /// returns `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_require_task(&self, task: &T) -> Option<(&RequireTaskStart<T>, &RequireTaskEnd<T, T::Output>)> {
    let start_data = self.find_map(|e| e.match_require_task_start(task));
    let end_data = self.find_map(|e| e.match_require_task_end(task));
    start_data.zip(end_data)
  }
  /// Finds the first require [start](Event::RequireTaskStart) and [end](Event::RequireTaskEnd) event for `task` and
  /// returns `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_require_task_range(&self, task: &T) -> Option<RangeInclusive<usize>> {
    self.first_require_task(task).map(|(s, e)| s.index..=e.index)
  }

  /// Returns `true` if any task was executed.
  pub fn any_execute(&self) -> bool {
    self.any(|e| e.is_execute())
  }
  /// Returns `true` if `task` was executed.
  pub fn any_execute_of(&self, task: &T) -> bool {
    self.any(|e| e.is_execute_of(task))
  }
  /// Returns `true` if `task` was executed exactly once.
  pub fn one_execute_of(&self, task: &T) -> bool {
    self.one(|e| e.match_execute_start(task).is_some())
  }

  /// Finds the first execute [start](Event::ExecuteStart) and [end](Event::ExecuteEnd) event for `task` and returns
  /// `Some((&start_data, &end_data))`, or `None` otherwise.
  pub fn first_execute(&self, task: &T) -> Option<(&ExecuteStart<T>, &ExecuteEnd<T, T::Output>)> {
    let start_data = self.find_map(|e| e.match_execute_start(task));
    let end_data = self.find_map(|e| e.match_execute_end(task));
    start_data.zip(end_data)
  }
  /// Finds the first execute [start](Event::ExecuteStart) and [end](Event::ExecuteEnd) event for `task` and returns
  /// `Some(start_data.index..=end_data.index)`, or `None` otherwise.
  pub fn first_execute_range(&self, task: &T) -> Option<RangeInclusive<usize>> {
    self.first_execute(task).map(|(s, e)| s.index..=e.index)
  }
}
