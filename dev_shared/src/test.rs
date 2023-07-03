use std::io::Stdout;

use rstest::fixture;
use tempfile::TempDir;

use ::pie::Task;
use ::pie::tracker::CompositeTracker;
use ::pie::tracker::event::EventTracker;
use ::pie::tracker::writing::WritingTracker;

/// Testing PIE with event tracking for testing and stdout writing for debugging.
pub type TestPie<T> = ::pie::Pie<T, <T as Task>::Output, CompositeTracker<EventTracker<T>, WritingTracker<Stdout, T>>>;

#[inline]
pub fn create_test_pie<T: Task>() -> TestPie<T> {
  let tracker = CompositeTracker(EventTracker::default(), WritingTracker::new_stdout_writer());
  TestPie::with_tracker(tracker)
}

// Fixtures

#[fixture]
#[inline]
pub fn pie<T: Task>() -> TestPie<T> {
  create_test_pie()
}

#[fixture]
#[inline]
pub fn temp_dir() -> TempDir {
  crate::fs::create_temp_dir()
}
