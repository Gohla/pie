use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;

use pie::Task;
use pie::tracker::NoopTracker;

/// Benchmarking PIE without tracking and more efficient `FxHasher` hasher
pub type BenchPie<T> = pie::Pie<T, <T as Task>::Output, NoopTracker, BuildHasherDefault<FxHasher>>;

pub fn create_bench_pie<T: Task>() -> BenchPie<T> {
  BenchPie::new(NoopTracker)
}
