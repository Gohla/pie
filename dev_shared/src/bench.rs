use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;

use pie::Task;
use pie::tracker::NoopTracker;

pub type Pie<T> = pie::Pie<T, NoopTracker<T>, BuildHasherDefault<FxHasher>>;

pub fn create_pie<T: Task>() -> Pie<T> {
  Pie::new(NoopTracker::default())
}
