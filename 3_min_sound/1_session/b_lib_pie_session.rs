
/// Main entry point into PIE, a sound and incremental programmatic build system.
pub struct Pie<T, O> {
  store: Store<T, O>,
}

impl<T: Task> Default for Pie<T, T::Output> {
  fn default() -> Self { Self { store: Store::default() } }
}

impl<T: Task> Pie<T, T::Output> {
  /// Creates a new build session. Only one session may be active at once, enforced via mutable (exclusive) borrow.
  pub fn new_session(&mut self) -> Session<T, T::Output> { Session::new(self) }
  /// Runs `f` inside a new build session.
  pub fn run_in_session<R>(&mut self, f: impl FnOnce(Session<T, T::Output>) -> R) -> R {
    let session = self.new_session();
    f(session)
  }
}

/// A session in which builds are executed.
pub struct Session<'p, T, O> {
  store: &'p mut Store<T, O>,
  current_executing_task: Option<TaskNode>,
  dependency_check_errors: Vec<io::Error>,
}

impl<'p, T: Task> Session<'p, T, T::Output> {
  fn new(pie: &'p mut Pie<T, T::Output>) -> Self {
    Self {
      store: &mut pie.store,
      current_executing_task: None,
      dependency_check_errors: Vec::default(),
    }
  }

  /// Requires `task`, returning its up-to-date output.
  pub fn require(&mut self, task: &T) -> T::Output {
    todo!("Create TopDownContext with this session, and require the task")
  }

  /// Gets all errors produced during dependency checks.
  pub fn dependency_check_errors(&self) -> &[io::Error] { &self.dependency_check_errors }
}
