use std::io::{BufWriter, Stdout};

use pie::{BottomUp, Pie, Session, Task};
use pie::tracker::CompositeTracker;
use pie::tracker::event::EventTracker;
use pie::tracker::writing::WritingTracker;

/// Testing tracker composed of an [`EventTracker`] for testing and stdout [`WritingTracker`] for debugging.
pub type TestTracker = CompositeTracker<EventTracker, WritingTracker<BufWriter<Stdout>>>;
pub fn new_test_tracker() -> TestTracker {
  CompositeTracker(EventTracker::default(), WritingTracker::with_stdout())
}

/// Testing [`Pie`] using [`TestTracker`].
pub type TestPie = Pie<TestTracker>;
pub fn new_test_pie() -> TestPie {
  Pie::with_tracker(new_test_tracker())
}

/// Testing extensions for [`TestPie`].
pub trait TestPieExt {
  /// Runs `run_func` in a new session, asserts that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  fn assert_in_session<R>(
    &mut self,
    run_func: impl FnOnce(&mut Session) -> R,
    test_assert_func: impl FnOnce(&EventTracker),
  ) -> R;


  /// Require `task` in a new session, assert that there are no dependency check errors, then runs `test_assert_func`
  /// on the event tracker for test assertion purposes.
  fn require_then_assert<T: Task>(
    &mut self,
    task: &T,
    test_assert_func: impl FnOnce(&EventTracker),
  ) -> T::Output {
    self.assert_in_session(|s| s.require(task), test_assert_func)
  }
  /// Require `task` in a new session, asserts that there are no dependency check errors.
  fn require<T: Task>(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |_| {})
  }

  /// Require `task` in a new session, then assert that it is not executed.
  fn require_then_assert_no_execute<T: Task>(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(!t.any_execute_of(task), "expected no execution of task {:?}, but it was executed", task),
    )
  }
  /// Require `task` in a new session, then assert that it is executed exactly once.
  fn require_then_assert_one_execute<T: Task>(&mut self, task: &T) -> T::Output {
    self.require_then_assert(task, |t|
      assert!(t.one_execute_of(task), "expected one execution of task {:?}, but it was not executed, or was executed more than once", task),
    )
  }


  fn bottom_up_build_then_assert(
    &mut self,
    bottom_up_func: impl FnOnce(&mut BottomUp),
    test_assert_func: impl FnOnce(&EventTracker),
  ) {
    self.assert_in_session(|s| {
      let mut bottom_up = s.bottom_up_build();
      bottom_up_func(&mut bottom_up);
      bottom_up.update_affected_tasks();
    }, test_assert_func)
  }
}
impl TestPieExt for TestPie {
  fn assert_in_session<R>(
    &mut self,
    run_func: impl FnOnce(&mut Session) -> R,
    test_func: impl FnOnce(&EventTracker),
  ) -> R {
    let mut session = self.new_session();
    let output = run_func(&mut session);
    let dependency_check_errors: Vec<_> = session.dependency_check_errors().collect();
    assert!(dependency_check_errors.is_empty(), "expected no dependency checking errors, but there are some: {:?}", dependency_check_errors);
    test_func(&self.tracker().0);
    output
  }
}
