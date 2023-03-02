use std::hash::BuildHasherDefault;

use criterion::{BatchSize, BenchmarkId, black_box, Criterion, criterion_group, criterion_main, Throughput};
use rustc_hash::FxHasher;

use dev_shared::{CommonTask, temp_dir};
use pie::Pie;
use pie::stamp::FileStamper;
use pie::tracker::NoopTracker;

pub fn bench_task_with_dependencies(c: &mut Criterion) {
  let mut g = c.benchmark_group("top-down check task with N dependencies");
  for (size, sample_size) in [(1000, 100), (10_000, 20), (100_000, 10)] {
    let num_dependencies = size * 3;
    g.throughput(Throughput::Elements(num_dependencies as u64));
    g.sample_size(sample_size);

    // Create task with N dependencies.
    let temp_dir = temp_dir();
    let task_with_file_deps = {
      let mut tasks = Vec::with_capacity(size);
      for i in 0..size {
        let path = temp_dir.path().join(format!("in{}.txt", i));
        tasks.push(CommonTask::to_lower_case(CommonTask::read_string_from_file(path, FileStamper::Modified)));
      }
      CommonTask::sequence(tasks)
    };
    let task_without_file_deps = {
      let mut tasks = Vec::with_capacity(size);
      for i in 0..size {
        tasks.push(CommonTask::to_lower_case(CommonTask::string_constant(format!("constant{}", i))));
      }
      CommonTask::sequence(tasks)
    };

    g.bench_function(BenchmarkId::new("with file dependencies", num_dependencies), |b| {
      b.iter_batched(
        || {
          let mut pie = Pie::<_, _, BuildHasherDefault<FxHasher>>::new(NoopTracker::default());
          // Require the task once, so all tasks are executed and cached.
          pie.run_in_session(|mut session| {
            session.require(&task_with_file_deps)
          });
          pie
        },
        |mut pie| {
          // Measure the time it takes to top-down check that nothing has changed.
          pie.run_in_session(|mut session| {
            black_box(session.require(&task_with_file_deps));
          });
        },
        BatchSize::LargeInput,
      );
    });
    g.bench_function(BenchmarkId::new("without file dependencies", num_dependencies), |b| {
      b.iter_batched(
        || {
          let mut pie = Pie::<_, _, BuildHasherDefault<FxHasher>>::new(NoopTracker::default());
          // Require the task once, so all tasks are executed and cached.
          pie.run_in_session(|mut session| {
            session.require(&task_without_file_deps)
          });
          pie
        },
        |mut pie| {
          // Measure the time it takes to top-down check that nothing has changed.
          pie.run_in_session(|mut session| {
            black_box(session.require(&task_without_file_deps));
          });
        },
        BatchSize::LargeInput,
      );
    });
  }
  g.finish();
}

criterion_group!(benches, bench_task_with_dependencies);
criterion_main!(benches);
