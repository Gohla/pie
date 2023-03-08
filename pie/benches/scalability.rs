use std::fs;
use std::path::{Path, PathBuf};

use criterion::{BatchSize, BenchmarkId, black_box, Criterion, criterion_group, criterion_main, Throughput};
use tempfile::TempDir;

use dev_shared::bench::create_pie;
use dev_shared::create_temp_dir;
use dev_shared::task::CommonTask;
use pie::stamp::FileStamper;

/// Show that bottom-up builds scale better than top-down builds due to bottom-up builds only checking the affected
/// region of the dependency graph.
pub fn top_down_vs_bottom_up_scalability(c: &mut Criterion) {
  fn create_tasks_and_paths(size: usize, temp_dir: &TempDir) -> (CommonTask, Vec<PathBuf>) {
    let mut tasks = Vec::with_capacity(size);
    let mut paths = Vec::with_capacity(size);
    for i in 0..size {
      let path = temp_dir.path().join(format!("in{}.txt", i));
      tasks.push(CommonTask::file_exists(path.clone()));
      paths.push(path);
    }
    (CommonTask::sequence(tasks), paths)
  }

  fn change_file<P: AsRef<Path>>(path: P) {
    let path = path.as_ref();
    if Path::exists(path) {
      fs::remove_file(path).unwrap();
    } else {
      fs::File::create(path).unwrap();
    }
  }

  fn change_files_percentage(paths: &Vec<PathBuf>, size: usize, percentage: f64) -> Vec<PathBuf> {
    let num_changes = size as f64 * percentage;
    let mut changed_paths = Vec::with_capacity(num_changes as usize);
    for path in paths.iter().step_by((size as f64 / num_changes) as usize) {
      change_file(&path);
      changed_paths.push(path.clone());
    }
    changed_paths
  }

  fn remove_files(paths: &Vec<PathBuf>) {
    for path in paths {
      if Path::exists(path) {
        fs::remove_file(path).unwrap();
      }
    }
  }

  let mut g = c.benchmark_group("top-down vs bottom-up scalability (N dependencies)");
  for (size, sample_size) in [(1000, 100), (10_000, 20), (100_000, 10)] {
    let num_dependencies = size * 2;
    g.throughput(Throughput::Elements(num_dependencies as u64));
    g.sample_size(sample_size);

    // Create task with N dependencies.
    let temp_dir = create_temp_dir();
    let (task, paths) = create_tasks_and_paths(size, &temp_dir);
    remove_files(&paths);

    // Top-down builds
    g.bench_function(BenchmarkId::new("top-down & no changes", num_dependencies), |b| {
      let mut pie = create_pie();
      pie.run_in_session(|mut session| { session.require(&task); });
      b.iter(|| {
        pie.run_in_session(|mut session| {
          black_box(session.require(&task));
        });
      });
    });
    let mut bench_top_down_changes = |ratio| {
      let percentage = (ratio * 100.0) as u64;
      g.bench_function(BenchmarkId::new(format!("top-down & {}% changes", percentage), num_dependencies), |b| {
        let mut pie = create_pie();
        pie.run_in_session(|mut session| { session.require(&task); });
        b.iter_batched(
          || {
            change_files_percentage(&paths, size, ratio);
          },
          |_| {
            pie.run_in_session(|mut session| {
              black_box(session.require(&task));
            });
          },
          BatchSize::PerIteration,
        );
      });
    };
    bench_top_down_changes(0.01);
    bench_top_down_changes(0.02);
    bench_top_down_changes(0.05);
    bench_top_down_changes(0.10);

    // Re-initialize files so bottom-up build starts with a fresh state.
    remove_files(&paths);

    // Bottom-up builds
    g.bench_function(BenchmarkId::new("bottom-up & no changes", num_dependencies), |b| {
      let mut pie = create_pie();
      pie.run_in_session(|mut session| { session.require(&task); });
      b.iter(|| {
        pie.run_in_session(|mut session| {
          session.update_affected_by([]);
        });
      });
    });
    let mut bench_bottom_up_changes = |ratio| {
      let percentage = (ratio * 100.0) as u64;
      g.bench_function(BenchmarkId::new(format!("bottom-up & {}% changes", percentage), num_dependencies), |b| {
        let mut pie = create_pie();
        pie.run_in_session(|mut session| { session.require(&task); });
        b.iter_batched(
          || {
            change_files_percentage(&paths, size, ratio)
          },
          |changed_files| {
            pie.run_in_session(|mut session| {
              session.update_affected_by(&changed_files);
            });
          },
          BatchSize::PerIteration,
        );
      });
    };
    bench_bottom_up_changes(0.01);
    bench_bottom_up_changes(0.02);
    bench_bottom_up_changes(0.05);
    bench_bottom_up_changes(0.10);
  }
  g.finish();
}

/// Show that file dependencies are slower than task dependencies (if task outputs are simple), due to system calls 
/// being more expensive than equality checks on task outputs.
pub fn file_dep_scaling(c: &mut Criterion) {
  fn create_task_with_file_deps(size: usize, temp_dir: &TempDir) -> CommonTask {
    let mut tasks = Vec::with_capacity(size);
    for i in 0..size {
      let path = temp_dir.path().join(format!("in{}.txt", i));
      tasks.push(CommonTask::to_lower_case(CommonTask::read_string_from_file(path.clone(), FileStamper::Modified)));
    }
    CommonTask::sequence(tasks)
  }

  fn create_task_without_file_deps(size: usize) -> CommonTask {
    let mut tasks = Vec::with_capacity(size);
    for i in 0..size {
      tasks.push(CommonTask::to_lower_case(CommonTask::string_constant(format!("constant{}", i))));
    }
    CommonTask::sequence(tasks)
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
    let mut pie = create_pie();
    pie.run_in_session(|mut session| {
      session.require(&task_without_file_deps)
    });

    b.iter(|| {
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
      pie.run_in_session(|mut session| {
        black_box(session.require(&task_with_file_deps));
      });
    });
  });

  g.finish();
}

criterion_group!(benches, top_down_vs_bottom_up_scalability, file_dep_scaling);
criterion_main!(benches);
