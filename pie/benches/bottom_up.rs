use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main, Throughput};

use dev_shared::bench::{create_pie, create_sequence_with_tolower_read_deps};
use dev_shared::create_temp_dir;

/// Show that bottom-up builds scale up to large dependency graphs, due to only the *affected part* of the dependency
/// graph being checked.
pub fn bench_task_with_deps_bottom_up_scaling(c: &mut Criterion) {
  let mut g = c.benchmark_group("bottom-up check task with N dependencies");
  for (size, sample_size) in [(1000, 100), (10_000, 100), (100_000, 100)] {
    let num_dependencies = size * 3;
    g.throughput(Throughput::Elements(num_dependencies as u64));
    g.sample_size(sample_size);

    // Create task with N dependencies.
    let temp_dir = create_temp_dir();
    let task = create_sequence_with_tolower_read_deps(size, &temp_dir);

    // Require the task once, so all tasks are executed and cached.
    let mut pie = create_pie();
    pie.run_in_session(|mut session| {
      session.require(&task);
    });

    g.bench_function(BenchmarkId::new("no changes", num_dependencies), |b| {
      b.iter(|| {
        // Measure the time it takes to bottom-up check that nothing has changed.
        pie.run_in_session(|mut session| {
          session.update_affected_by([]);
        });
      });
    });
  }
  g.finish();
}

criterion_group!(benches, bench_task_with_deps_bottom_up_scaling);
criterion_main!(benches);
