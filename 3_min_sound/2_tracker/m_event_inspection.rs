
impl<T: Task> EventTracker<T> {
  /// Returns a slice over all events.
  pub fn slice(&self) -> &[Event<T>] {
    &self.events
  }
  /// Returns an iterator over all events.
  pub fn iter(&self) -> impl Iterator<Item=&Event<T>> {
    self.events.iter()
  }

  /// Returns `true` if `predicate` returns `true` for any event.
  pub fn any(&self, predicate: impl FnMut(&Event<T>) -> bool) -> bool {
    self.iter().any(predicate)
  }
  /// Returns the number of times `predicate` returns `true`.
  pub fn count(&self, predicate: impl FnMut(&&Event<T>) -> bool) -> usize {
    self.iter().filter(predicate).count()
  }
  /// Returns `true` if `predicate` returns `true` for exactly one event.
  pub fn one(&self, predicate: impl FnMut(&&Event<T>) -> bool) -> bool {
    self.count(predicate) == 1
  }

  /// Returns `Some(index)` for the first event `e` where `predicate(e)` returns `true`, or `None` otherwise.
  pub fn index_of(&self, predicate: impl FnMut(&Event<T>) -> bool) -> Option<usize> {
    self.iter().position(predicate)
  }
  /// Returns `Some(v)` for the first event `e` where `f(e)` returns `Some(v)`, or `None` otherwise.
  pub fn find_map<R>(&self, f: impl FnMut(&Event<T>) -> Option<&R>) -> Option<&R> {
    self.iter().find_map(f)
  }
  /// Returns `Some((index, v))` for the first event `e` where `f(e)` returns `Some(v)`, or `None` otherwise.
  pub fn index_find_map<R>(&self, mut f: impl FnMut(&Event<T>) -> Option<&R>) -> Option<(usize, &R)> {
    self.iter().enumerate().find_map(|(i, e)| f(e).map(|o| (i, o)))
  }


  /// Finds the first [required file event](Event::RequiredFile) for `path` and returns its stamp as `Some(stamp)`, or 
  /// `None` if no event was found.
  pub fn stamp_of_first_required_file(&self, path: &PathBuf) -> Option<&FileStamp> {
    self.find_map(|e| e.stamp_of_required_file(path))
  }

  /// Returns `true` if any task was executed.
  pub fn any_execution(&self) -> bool {
    self.any(|e| e.is_execution())
  }
  /// Returns `true` if `task` was executed.
  pub fn any_execution_of(&self, task: &T) -> bool {
    self.any(|e| e.is_execution_of(task))
  }
  /// Returns `true` if `task` was executed exactly once.
  pub fn one_execute_of(&self, task: &T) -> bool {
    self.one(|e| e.is_execute(task))
  }

  /// Finds the first [task execute event](Event::Execute) for `task` and returns its index as `Some(index)`, or `None` 
  /// if no event was found.
  pub fn index_of_first_execute(&self, task: &T) -> Option<usize> {
    self.index_of(|e| e.is_execute(task))
  }
  /// Finds the first [task executed event](Event::Executed) for `task` and returns its index as `Some(index)`, or 
  /// `None` if no event was found.
  pub fn index_of_first_executed(&self, task: &T) -> Option<usize> {
    self.index_of(|e| e.output_of_executed(task).is_some())
  }
  /// Finds the first [task executed event](Event::Executed) for `task` and returns its index and output as 
  /// `Some((index, output))`, or `None` if no event was found.
  pub fn index_output_of_first_executed(&self, task: &T) -> Option<(usize, &T::Output)> {
    self.index_find_map(|e| e.output_of_executed(task))
  }
}

impl<T: Task> Event<T> {
  /// Returns `Some(stamp)` if this is a [required file event](Event::RequiredFile) for file at `path`, or `None` 
  /// otherwise.
  pub fn stamp_of_required_file(&self, path: &PathBuf) -> Option<&FileStamp> {
    match self {
      Event::RequiredFile { path: p, stamp, .. } if p == path => Some(stamp),
      _ => None,
    }
  }

  /// Returns `true` if this is a task execution (execute or executed) event.
  pub fn is_execution(&self) -> bool {
    match self {
      Event::Execute { .. } => true,
      Event::Executed { .. } => true,
      _ => false,
    }
  }
  /// Returns `true` if this is an execution (execute or executed) event for `task`.
  pub fn is_execution_of(&self, task: &T) -> bool {
    match self {
      Event::Execute { task: t } if t == task => true,
      Event::Executed { task: t, .. } if t == task => true,
      _ => false,
    }
  }
  /// Returns `true` if this is a [task execute event](Event::Execute) for `task`.
  pub fn is_execute(&self, task: &T) -> bool {
    match self {
      Event::Execute { task: t } if t == task => true,
      _ => false,
    }
  }
  /// Returns `Some(output)` if this is a [task executed event](Event::Executed) for `task`, or `None` otherwise.
  pub fn output_of_executed(&self, task: &T) -> Option<&T::Output> {
    match self {
      Event::Executed { task: t, output: o } if t == task => Some(o),
      _ => None,
    }
  }
}
