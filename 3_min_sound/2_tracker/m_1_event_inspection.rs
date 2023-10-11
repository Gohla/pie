
impl<T: Task> Event<T, T::Output> {
  /// Returns `Some(&data)` if this is a [require file end event](Event::RequireFileEnd) for file at `path`, or `None`
  /// otherwise.
  pub fn match_require_file_end(&self, path: impl AsRef<Path>) -> Option<&RequireFileEnd> {
    let path = path.as_ref();
    match self {
      Event::RequireFileEnd(data) if data.path == path => Some(data),
      _ => None,
    }
  }

  /// Returns `Some(&data)` if this is a [require start event](Event::RequireTaskStart) for `task`, or `None` otherwise.
  pub fn match_require_task_start(&self, task: &T) -> Option<&RequireTaskStart<T>> {
    match self {
      Event::RequireTaskStart(data) if data.task == *task => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is a [require end event](Event::RequireTaskEnd) for `task`, or `None` otherwise.
  pub fn match_require_task_end(&self, task: &T) -> Option<&RequireTaskEnd<T, T::Output>> {
    match self {
      Event::RequireTaskEnd(data) if data.task == *task => Some(data),
      _ => None,
    }
  }

  /// Returns `true` if this is a task execute [start](Event::ExecuteStart) or [end](Event::ExecuteEnd) event.
  pub fn is_execute(&self) -> bool {
    match self {
      Event::ExecuteStart(_) | Event::ExecuteEnd(_) => true,
      _ => false,
    }
  }
  /// Returns `true` if this is an execute [start](Event::ExecuteStart) or [end](Event::ExecuteEnd) event for `task`.
  pub fn is_execute_of(&self, task: &T) -> bool {
    match self {
      Event::ExecuteStart(ExecuteStart { task: t, .. }) |
      Event::ExecuteEnd(ExecuteEnd { task: t, .. }) if t == task => true,
      _ => false,
    }
  }
  /// Returns `Some(&data)` if this is an [execute start event](Event::ExecuteStart) for `task`, or `None` otherwise.
  pub fn match_execute_start(&self, task: &T) -> Option<&ExecuteStart<T>> {
    match self {
      Event::ExecuteStart(data) if data.task == *task => Some(data),
      _ => None,
    }
  }
  /// Returns `Some(&data)` if this is an [execute end event](Event::ExecuteEnd) for `task`, or `None` otherwise.
  pub fn match_execute_end(&self, task: &T) -> Option<&ExecuteEnd<T, T::Output>> {
    match self {
      Event::ExecuteEnd(data) if data.task == *task => Some(data),
      _ => None,
    }
  }
}
