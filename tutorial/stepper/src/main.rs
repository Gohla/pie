use std::fs::{OpenOptions, read_to_string};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
  let temp_directory = tempfile::tempdir().expect("failed to create temporary directory");
  let stepper = Stepper::new("../src/", temp_directory.path());
  stepper.step_additions([
    Addition::new("api/Cargo.toml", "Cargo.toml"),
    Addition::new("api/lib.rs", "src/lib.rs")
  ]);
  stepper.step_addition(Addition::new("api/non_incremental.rs", "src/lib.rs"));
  stepper.step_addition(Addition::new("api/non_incremental_test_1.rs", "src/lib.rs"));
  stepper.step_diff(Diff::new("api/non_incremental_test_1.rs", "api/non_incremental_test_2.rs", "src/lib.rs"));
}

struct Stepper {
  source_root_directory: PathBuf,
  destination_root_directory: PathBuf,
}

impl Stepper {
  pub fn new(source_root_directory: impl Into<PathBuf>, destination_root_directory: impl Into<PathBuf>) -> Self {
    let source_root_directory = source_root_directory.into();
    let destination_root_directory = destination_root_directory.into();
    Self { source_root_directory, destination_root_directory }
  }
}

struct Addition {
  addition_file_path: PathBuf,
  destination_file_path: PathBuf,
}

impl Addition {
  pub fn new(addition_file_path: impl Into<PathBuf>, destination_file_path: impl Into<PathBuf>) -> Self {
    let addition_file_path = addition_file_path.into();
    let destination_file_path = destination_file_path.into();
    Self { addition_file_path, destination_file_path }
  }
}

impl Stepper {
  pub fn step_addition(&self, addition: Addition) {
    self.step_additions([addition]);
  }

  pub fn step_additions(&self, additions: impl IntoIterator<Item=Addition>) {
    for addition in additions {
      let addition_file_path = self.source_root_directory.join(&addition.addition_file_path);
      let destination_file_path = self.destination_root_directory.join(&addition.destination_file_path);
      println!("Appending {} to {}", addition_file_path.display(), destination_file_path.display());
      let addition_text = read_to_string(&addition_file_path)
        .expect("failed to read addition file to string");
      std::fs::create_dir_all(destination_file_path.parent().unwrap())
        .expect("failed to create parent directories for destination file");
      let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(destination_file_path)
        .expect("failed to open file for appending");
      let line_comment_start = line_comment_start(&addition_file_path);
      write!(file, "{} Additions from {}\n\n", line_comment_start, addition_file_path.display())
        .expect("failed to append header to destination file");
      write!(file, "{}\n\n", addition_text)
        .expect("failed to append to destination file");
    }
    self.cargo_test();
  }
}

struct Diff {
  original_file_path: PathBuf,
  modified_file_path: PathBuf,
  destination_file_path: PathBuf,
}

impl Diff {
  pub fn new(original_file_path: impl Into<PathBuf>, modified_file_path: impl Into<PathBuf>, destination_file_path: impl Into<PathBuf>) -> Self {
    let original_file_path = original_file_path.into();
    let modified_file_path = modified_file_path.into();
    let destination_file_path = destination_file_path.into();
    Self { original_file_path, modified_file_path, destination_file_path }
  }
}


impl Stepper {
  pub fn step_diff(&self, diff: Diff) {
    let original_file_path = self.source_root_directory.join(&diff.original_file_path);
    let modified_file_path = self.source_root_directory.join(&diff.modified_file_path);
    let destination_file_path = self.destination_root_directory.join(&diff.destination_file_path);
    println!("Diffing {} and {} into a patch, applying patch to {}", original_file_path.display(), modified_file_path.display(), destination_file_path.display());
    diff_and_apply(&original_file_path, &modified_file_path, &destination_file_path);
    self.cargo_test();
  }
}

impl Stepper {
  fn cargo_test(&self) {
    let mut command = Command::new("cargo");
    command
      .args(["+stable", "test"])
      .current_dir(&self.destination_root_directory);
    println!("Running {:?}", command);
    command
      .spawn()
      .expect("failed to start cargo test")
      .wait()
      .expect("cargo test failed");
  }
}

fn line_comment_start(file: impl AsRef<Path>) -> &'static str {
  if let Some(ext) = file.as_ref().extension() {
    if let Some(ext) = ext.to_str() {
      match ext {
        "rs" => return "//",
        "toml" => return " # ",
        _ => {}
      }
    }
  }
  return "//";
}

fn diff_and_apply(original_file_path: impl AsRef<Path>, modified_file_path: impl AsRef<Path>, destination_file_path: impl AsRef<Path>) {
  let original_text = read_to_string(original_file_path)
    .expect("failed to read original file text");
  let modified_text = read_to_string(modified_file_path)
    .expect("failed to read modified file text");
  let patch = diffy::create_patch(&original_text, &modified_text);

  let destination_text = read_to_string(destination_file_path.as_ref())
    .expect("failed to read destination file text");
  let destination_text = diffy::apply(&destination_text, &patch)
    .expect("failed to apply diff");

  let mut destination_file = OpenOptions::new()
    .write(true)
    .create(true)
    .open(destination_file_path.as_ref())
    .expect("failed to open destination file for writing");
  destination_file.write(destination_text.as_bytes())
    .expect("failed to write to destination file");
}
