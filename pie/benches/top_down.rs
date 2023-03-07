use criterion::{BenchmarkId, black_box, Criterion, criterion_group, criterion_main, Throughput};

use dev_shared::bench::{create_pie, create_sequence_with_tolower_constant_deps, create_sequence_with_tolower_read_deps};
use dev_shared::create_temp_dir;

/// Show that top-down builds do not scale up to large dependency graphs, due to the entire dependency graph being 
/// checked.
pub fn bench_task_with_deps_top_down_scaling(c: &mut Criterion) {
  let mut g = c.benchmark_group("top-down check task with N dependencies");
  for (size, sample_size) in [(1000, 100), (10_000, 20), (100_000, 10)] {
    let num_dependencies = size * 3;
    g.throughput(Throughput::Elements(num_dependencies as u64));
    g.sample_size(sample_size);

    // Create task with N dependencies.
    let temp_dir = create_temp_dir();
    let task = create_sequence_with_tolower_read_deps(size, &temp_dir);

    g.bench_function(BenchmarkId::new("no changes", num_dependencies), |b| {
      // Require the task once, so all tasks are executed and cached.
      let mut pie = create_pie();
      pie.run_in_session(|mut session| {
        session.require(&task)
      });

      b.iter(|| {
        // Measure the time it takes to top-down check that nothing has changed.
        pie.run_in_session(|mut session| {
          black_box(session.require(&task));
        });
      });
    });
  }
  g.finish();
}

/// Show that file dependencies are slower than task dependencies (if task outputs are simple), due to system calls 
/// being more expensive than equality checks on task outputs.
pub fn bench_task_with_file_deps_and_without(c: &mut Criterion) {
  let mut g = c.benchmark_group("task dependencies vs file dependencies");

  let size = 100_000;
  let num_dependencies = size * 3;
  g.throughput(Throughput::Elements(num_dependencies as u64));
  g.sample_size(10);

  // Create task with N dependencies.
  let temp_dir = create_temp_dir();
  let task_with_file_deps = create_sequence_with_tolower_read_deps(size, &temp_dir);
  let task_without_file_deps = create_sequence_with_tolower_constant_deps(size);

  g.bench_function(BenchmarkId::new("without file dependencies", num_dependencies), |b| {
    // Require the task once, so all tasks are executed and cached.
    let mut pie = create_pie();
    pie.run_in_session(|mut session| {
      session.require(&task_without_file_deps)
    });

    b.iter(|| {
      // Measure without file deps.
      pie.run_in_session(|mut session| {
        black_box(session.require(&task_without_file_deps));
      });
    });
  });

  g.bench_function(BenchmarkId::new("with file dependencies", num_dependencies), |b| {
    // Require the task once, so all tasks are executed and cached.
    let mut pie = create_pie();
    pie.run_in_session(|mut session| {
      session.require(&task_with_file_deps)
    });

    b.iter(|| {
      // Measure with file deps.
      pie.run_in_session(|mut session| {
        black_box(session.require(&task_with_file_deps));
      });
    });
  });

  g.finish();
}

criterion_group!(benches, bench_task_with_deps_top_down_scaling, bench_task_with_file_deps_and_without);
criterion_main!(benches);
