use criterion::{BenchmarkId, black_box, Criterion, criterion_group, criterion_main, Throughput};
use tempfile::TempDir;

use dev_shared::bench::create_bench_pie;
use dev_shared::fs::create_temp_dir;
use dev_shared::task::*;
use pie::stamp::FileStamper;

/// Show that file dependencies are slower than task dependencies (if task outputs are simple), due to system calls 
/// being more expensive than equality checks on task outputs.
pub fn file_dependency_scalability(c: &mut Criterion) {
  fn create_task_with_file_deps(size: usize, temp_dir: &TempDir) -> CommonTask {
    let mut tasks = Vec::with_capacity(size);
    for i in 0..size {
      let path = temp_dir.path().join(format!("in{}.txt", i));
      tasks.push(ToLowerCase::new(ReadStringFromFile::new(path, FileStamper::Modified)));
    }
    Sequence::new(tasks)
  }

  fn create_task_without_file_deps(size: usize) -> CommonTask {
    let mut tasks = Vec::with_capacity(size);
    for i in 0..size {
      tasks.push(ToLowerCase::new(StringConstant::new(format!("constant{}", i))));
    }
    Sequence::new(tasks)
  }

  let mut g = c.benchmark_group("task dependencies vs file dependencies");

  let size = 100_000;
  let num_dependencies = size * 3;
  g.throughput(Throughput::Elements(num_dependencies as u64));
  g.sample_size(10);

  // Create task with N dependencies.
  let temp_dir = create_temp_dir();
  let task_with_file_deps = create_task_with_file_deps(size, &temp_dir);
  let task_without_file_deps = create_task_without_file_deps(size);

  g.bench_function(BenchmarkId::new("without file dependencies", num_dependencies), |b| {
    // Require the task once, so all tasks are executed and cached.
    let mut pie = create_bench_pie();
    pie.run_in_session(|mut session| {
      let _ = session.require(&task_without_file_deps);
    });

    b.iter(|| {
      pie.run_in_session(|mut session| {
        let _ = black_box(session.require(&task_without_file_deps));
      });
    });
  });

  g.bench_function(BenchmarkId::new("with file dependencies", num_dependencies), |b| {
    // Require the task once, so all tasks are executed and cached.
    let mut pie = create_bench_pie();
    pie.run_in_session(|mut session| {
      let _ = session.require(&task_with_file_deps);
    });

    b.iter(|| {
      pie.run_in_session(|mut session| {
        let _ = black_box(session.require(&task_with_file_deps));
      });
    });
  });

  g.finish();
}

criterion_group!(benches,  file_dependency_scalability);
criterion_main!(benches);
