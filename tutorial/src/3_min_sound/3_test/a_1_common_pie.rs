use std::io::{BufWriter, ErrorKind, Stdout};

use pie::{Context, Pie, Task};
use pie::tracker::CompositeTracker;
use pie::tracker::event::EventTracker;
use pie::tracker::writing::WritingTracker;

/// Testing tracker composed of an [`EventTracker`] for testing and stdout [`WritingTracker`] for debugging.
pub type TestTracker<T> = CompositeTracker<EventTracker<T, <T as Task>::Output>, WritingTracker<BufWriter<Stdout>>>;
pub fn test_tracker<T: Task>() -> TestTracker<T> {
  CompositeTracker(EventTracker::default(), WritingTracker::with_stdout())
}

/// Testing [`Pie`] using [`TestTracker`].
pub type TestPie<T> = Pie<T, <T as Task>::Output, TestTracker<T>>;
pub fn test_pie<T: Task>() -> TestPie<T> {
  TestPie::with_tracker(test_tracker())
}
