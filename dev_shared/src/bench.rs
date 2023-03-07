use std::hash::BuildHasherDefault;

use rustc_hash::FxHasher;
use tempfile::TempDir;

use pie::stamp::FileStamper;
use pie::Task;
use pie::tracker::NoopTracker;

use crate::task::CommonTask;

pub type Pie<T> = pie::Pie<T, NoopTracker<T>, BuildHasherDefault<FxHasher>>;

pub fn create_pie<T: Task>() -> Pie<T> {
  Pie::new(NoopTracker::default())
}

pub fn create_sequence_with_tolower_constant_deps(size: usize) -> CommonTask {
  let mut tasks = Vec::with_capacity(size);
  for i in 0..size {
    tasks.push(CommonTask::to_lower_case(CommonTask::string_constant(format!("constant{}", i))));
  }
  CommonTask::sequence(tasks)
}

pub fn create_sequence_with_tolower_read_deps(size: usize, temp_dir: &TempDir) -> CommonTask {
  let mut tasks = Vec::with_capacity(size);
  for i in 0..size {
    let path = temp_dir.path().join(format!("in{}.txt", i));
    tasks.push(CommonTask::to_lower_case(CommonTask::read_string_from_file(path, FileStamper::Modified)));
  }
  CommonTask::sequence(tasks)
}
