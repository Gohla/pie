use std::fs;
use std::path::{Path, PathBuf};

use criterion::{BatchSize, BenchmarkId, black_box, Criterion, criterion_group, criterion_main, Throughput};
use tempfile::TempDir;

use dev_shared::bench::create_pie;
use dev_shared::create_temp_dir;
use dev_shared::task::CommonTask;
use pie::stamp::FileStamper;

fn create_sequence_with_tolower_constant_deps(size: usize) -> CommonTask {
  let mut tasks = Vec::with_capacity(size);
  for i in 0..size {
    tasks.push(CommonTask::to_lower_case(CommonTask::string_constant(format!("constant{}", i))));
  }
  CommonTask::sequence(tasks)
}

fn create_sequence_with_tolower_read_deps(size: usize, temp_dir: &TempDir) -> (CommonTask, Vec<PathBuf>) {
  let mut tasks = Vec::with_capacity(size);
  let mut paths = Vec::with_capacity(size);
  for i in 0..size {
    let path = temp_dir.path().join(format!("in{}.txt", i));
    tasks.push(CommonTask::to_lower_case(CommonTask::read_string_from_file(path.clone(), FileStamper::Modified)));
    paths.push(path);
  }
  (CommonTask::sequence(tasks), paths)
}

fn initialize_files(paths: &Vec<PathBuf>) {
  for path in paths {
    fs::write(&path, "a").unwrap();
  }
}

fn change_file<P: AsRef<Path>>(path: P) {
  let mut str = fs::read_to_string(&path).unwrap_or_default();
  // Use lowercase characters here to simulate early cutoff; files will have to be read again when changed, but 
  // to_lower_case tasks do not.
  if str.is_empty() {
    str.push('a');
  } else {
    let mut char = str.remove(0);
    if char == 'a' {
      char = 'b';
    } else {
      char = 'a';
    }
    str.push(char);
  }
  fs::write(&path, str).unwrap();
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

/// Show that bottom-up builds scale better than top-down builds due to bottom-up builds only checking the affected
/// region of the dependency graph.
pub fn top_down_vs_bottom_up_scalability(c: &mut Criterion) {
  let mut g = c.benchmark_group("top-down vs bottom-up scalability (N dependencies)");
  for (size, sample_size) in [(1000, 100), (10_000, 20), (100_000, 10)] {
    let num_dependencies = size * 3;
    g.throughput(Throughput::Elements(num_dependencies as u64));
    g.sample_size(sample_size);

    // Create task with N dependencies.
    let temp_dir = create_temp_dir();
    let (task, paths) = create_sequence_with_tolower_read_deps(size, &temp_dir);
    initialize_files(&paths);

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
    g.bench_function(BenchmarkId::new("top-down & 1% changes", num_dependencies), |b| {
      let mut pie = create_pie();
      pie.run_in_session(|mut session| { session.require(&task); });
      b.iter_batched(
        || {
          change_files_percentage(&paths, size, 0.01);
        },
        |_| {
          pie.run_in_session(|mut session| {
            black_box(session.require(&task));
          });
        },
        BatchSize::PerIteration,
      );
    });
    g.bench_function(BenchmarkId::new("top-down & 10% changes", num_dependencies), |b| {
      let mut pie = create_pie();
      pie.run_in_session(|mut session| { session.require(&task); });
      b.iter_batched(
        || {
          change_files_percentage(&paths, size, 0.10);
        },
        |_| {
          pie.run_in_session(|mut session| {
            black_box(session.require(&task));
          });
        },
        BatchSize::PerIteration,
      );
    });

    // Re-initialize files so bottom-up build starts with a fresh state.
    initialize_files(&paths);

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
    g.bench_function(BenchmarkId::new("bottom-up & 1% changes", num_dependencies), |b| {
      let mut pie = create_pie();
      pie.run_in_session(|mut session| { session.require(&task); });
      b.iter_batched(
        || {
          change_files_percentage(&paths, size, 0.01)
        },
        |changed_files| {
          pie.run_in_session(|mut session| {
            session.update_affected_by(&changed_files);
          });
        },
        BatchSize::PerIteration,
      );
    });
    g.bench_function(BenchmarkId::new("bottom-up & 10% changes", num_dependencies), |b| {
      let mut pie = create_pie();
      pie.run_in_session(|mut session| { session.require(&task); });
      b.iter_batched(
        || {
          change_files_percentage(&paths, size, 0.10)
        },
        |changed_files| {
          pie.run_in_session(|mut session| {
            session.update_affected_by(&changed_files);
          });
        },
        BatchSize::PerIteration,
      );
    });
  }
  g.finish();
}

/// Show that file dependencies are slower than task dependencies (if task outputs are simple), due to system calls 
/// being more expensive than equality checks on task outputs.
pub fn file_dep_scaling(c: &mut Criterion) {
  let mut g = c.benchmark_group("task dependencies vs file dependencies");

  let size = 100_000;
  let num_dependencies = size * 3;
  g.throughput(Throughput::Elements(num_dependencies as u64));
  g.sample_size(10);

  // Create task with N dependencies.
  let temp_dir = create_temp_dir();
  let (task_with_file_deps, _) = create_sequence_with_tolower_read_deps(size, &temp_dir);
  let task_without_file_deps = create_sequence_with_tolower_constant_deps(size);

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
