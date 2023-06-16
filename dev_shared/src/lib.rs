use std::hash::BuildHasherDefault;
use std::io::Stdout;

use rustc_hash::FxHasher;

use pie::Task;
use pie::tracker::{CompositeTracker, NoopTracker};
use pie::tracker::event::EventTracker;
use pie::tracker::writing::WritingTracker;

pub mod fs;
pub mod task;
pub mod assert;
pub mod check;

/// Testing PIE with event tracking for testing purposes, and stdout writing for debugging.
pub type TestPie<T> = pie::Pie<T, <T as Task>::Output, CompositeTracker<EventTracker<T>, WritingTracker<Stdout, T>>>;

pub fn create_test_pie<T: Task>() -> TestPie<T> {
  let tracker = CompositeTracker(EventTracker::default(), WritingTracker::new_stdout_writer());
  TestPie::with_tracker(tracker)
}

/// Benchmarking PIE without tracking and more efficient `FxHasher` hasher
pub type BenchPie<T> = pie::Pie<T, <T as Task>::Output, NoopTracker<T>, BuildHasherDefault<FxHasher>>;

pub fn create_bench_pie<T: Task>() -> BenchPie<T> {
  BenchPie::new(NoopTracker::default())
}
