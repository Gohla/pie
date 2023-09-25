
/// Testing extensions for [`TestPie`].
pub trait TestPieExt<T: Task> {
  /// Require `task` in a new session, assert that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  fn require_then_assert(
    &mut self,
    task: &T,
    test_assert_func: impl FnOnce(&EventTracker<T, T::Output>),
  ) -> T::Output;
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
