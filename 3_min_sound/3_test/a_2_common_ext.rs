
/// Testing extensions for [`TestPie`].
pub trait TestPieExt<T: Task> {
  /// Require `task` in a new session, assert that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  fn require_then_assert(
    &mut self,
    task: &T,
    test_assert_func: impl FnOnce(&EventTracker<T, T::Output>),
  ) -> T::Output;

  /// Require `task` in a new session, asserts that there are no dependency check errors.
  fn require(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |_| {})
  }

  /// Require `task` in a new session, then assert that it is not executed.
  fn require_then_assert_no_execute(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(!t.any_execute_of(task), "expected no execution of task {:?}, but it was executed", task),
    )
  }
  /// Require `task` in a new session, then assert that it is executed exactly once.
  fn require_then_assert_one_execute(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(t.one_execute_of(task), "expected one execution of task {:?}, but it was not executed, or was executed more than once", task),
    )
  }
}
impl<T: Task> TestPieExt<T> for TestPie<T> {
  fn require_then_assert(&mut self, task: &T, test_assert_func: impl FnOnce(&EventTracker<T, T::Output>)) -> T::Output {
    let mut session = self.new_session();
    let output = session.require(task);
    assert!(session.dependency_check_errors().is_empty(), "expected no dependency checking errors, but there are \
    dependency checking errors: {:?}", session.dependency_check_errors());
    test_assert_func(&self.tracker().0);
    output
  }
}
