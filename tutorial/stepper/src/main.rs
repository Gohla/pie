use std::error::Error;
use std::fs;
use std::fs::{File, OpenOptions, read_to_string};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use diffy::Patch;

fn main() {
  let temp_directory = tempfile::tempdir().expect("failed to create temporary directory");
  let stepper = Stepper::new("../src/", temp_directory.path(), "out");
  stepper.apply_modifications([
    AddToFile::new("api/0_Cargo.toml", "Cargo.toml"),
    AddToFile::new("api/0_api.rs", "src/lib.rs"),
  ]);
  stepper.apply_modifications([
    CreateDiffAndApply::new("api/0_api.rs", "api/1_context_module.rs", "src/lib.rs", "1_context_module.rs.diff"),
    AddToFile::new("api/2_non_incremental_module.rs", "src/context/mod.rs"),
    CreateFile::new("src/context/non_incremental.rs")
  ]);
  stepper.apply_modification(AddToFile::new("api/3_non_incremental_context.rs", "src/context/non_incremental.rs"));
  stepper.apply_modification(AddToFile::new("api/4_test_1.rs", "src/context/non_incremental.rs"));
  stepper.apply_modification(CreateDiffAndApply::new("api/4_test_1.rs", "api/5_test_2.rs", "src/context/non_incremental.rs", "5_test_2.rs.diff"));
}

struct Stepper {
  source_root_directory: PathBuf,
  destination_root_directory: PathBuf,
  diff_output_root_directory: PathBuf,
}

impl Stepper {
  pub fn new(
    source_root_directory: impl Into<PathBuf>,
    destination_root_directory: impl Into<PathBuf>,
    diff_output_root_directory: impl Into<PathBuf>,
  ) -> Self {
    let source_root_directory = source_root_directory.into();
    let destination_root_directory = destination_root_directory.into();
    let diff_output_root_directory = diff_output_root_directory.into();
    Self { source_root_directory, destination_root_directory, diff_output_root_directory }
  }
}


// Apply modifications

enum Modification {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  CreateDiffAndApply(CreateDiffAndApply),
}

impl Stepper {
  pub fn apply_modifications(&self, modifications: impl IntoIterator<Item=Modification>) {
    for modification in modifications {
      match modification {
        Modification::CreateFile(create_file) => self.create_file(create_file),
        Modification::AddToFile(add_to_file) => self.add_to_file(add_to_file),
        Modification::CreateDiffAndApply(create_diff_and_apply) => self.create_diff_and_apply(create_diff_and_apply),
      }
    }
    self.cargo_test();
  }

  pub fn apply_modification(&self, modification: Modification) {
    self.apply_modifications([modification])
  }

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


// Create file

struct CreateFile {
  file_path: PathBuf,
}

impl CreateFile {
  pub fn new(file_path: impl Into<PathBuf>) -> Modification {
    let file_path = file_path.into();
    Modification::CreateFile(Self { file_path })
  }
}

impl Stepper {
  fn create_file(&self, create_file: CreateFile) {
    let file_path = self.destination_root_directory.join(&create_file.file_path);
    println!("Creating empty file {}", file_path.display());
    open_writable_file(&file_path, true)
      .expect("failed to create empty file");
  }
}


// Add (append) to file

struct AddToFile {
  addition_file_path: PathBuf,
  destination_file_path: PathBuf,
}

impl AddToFile {
  pub fn new(addition_file_path: impl Into<PathBuf>, destination_file_path: impl Into<PathBuf>) -> Modification {
    let addition_file_path = addition_file_path.into();
    let destination_file_path = destination_file_path.into();
    Modification::AddToFile(Self { addition_file_path, destination_file_path })
  }
}

impl Stepper {
  fn add_to_file(&self, add_to_file: AddToFile) {
    let addition_file_path = self.source_root_directory.join(&add_to_file.addition_file_path);
    let destination_file_path = self.destination_root_directory.join(&add_to_file.destination_file_path);
    println!("Appending {} to {}", addition_file_path.display(), destination_file_path.display());

    let addition_text = read_to_string(&addition_file_path)
      .expect("failed to read addition file to string");
    let mut file = open_writable_file(&destination_file_path, true)
      .expect("failed to open writable file");
    let line_comment_start = line_comment_start(&addition_file_path);
    write!(file, "{} Additions from {}\n\n", line_comment_start, addition_file_path.display())
      .expect("failed to append header to destination file");
    write!(file, "{}\n\n", addition_text)
      .expect("failed to append to destination file");
  }
}


// Create diff and apply it

struct CreateDiffAndApply {
  original_file_path: PathBuf,
  modified_file_path: PathBuf,
  destination_file_path: PathBuf,
  diff_output_file_path: PathBuf,
}

impl CreateDiffAndApply {
  pub fn new(
    original_file_path: impl Into<PathBuf>,
    modified_file_path: impl Into<PathBuf>,
    destination_file_path: impl Into<PathBuf>,
    diff_output_file_path: impl Into<PathBuf>,
  ) -> Modification {
    let original_file_path = original_file_path.into();
    let modified_file_path = modified_file_path.into();
    let destination_file_path = destination_file_path.into();
    let diff_output_file_path = diff_output_file_path.into();
    Modification::CreateDiffAndApply(Self { original_file_path, modified_file_path, destination_file_path, diff_output_file_path })
  }
}

impl Stepper {
  pub fn create_diff_and_apply(&self, create_diff_and_apply: CreateDiffAndApply) {
    let original_file_path = self.source_root_directory.join(&create_diff_and_apply.original_file_path);
    let modified_file_path = self.source_root_directory.join(&create_diff_and_apply.modified_file_path);
    let destination_file_path = self.destination_root_directory.join(&create_diff_and_apply.destination_file_path);
    let diff_output_file_path = self.diff_output_root_directory.join(create_diff_and_apply.diff_output_file_path);
    println!("Diffing {} and {}, applying diff to {}, writing diff to {}", original_file_path.display(), modified_file_path.display(), destination_file_path.display(), diff_output_file_path.display());

    let original_text = read_to_string(original_file_path)
      .expect("failed to read original file text");
    let modified_text = read_to_string(modified_file_path)
      .expect("failed to read modified file text");
    let patch = diffy::create_patch(&original_text, &modified_text);
    let mut diff_output_file = open_writable_file(diff_output_file_path, false)
      .expect("failed to open diff output file");
    diff_output_file.write_all(&patch.to_bytes())
      .expect("failed to write to diff output file");

    apply_diff(patch, &destination_file_path);
  }
}


// Helper functions

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

fn open_writable_file(file_path: impl AsRef<Path>, append: bool) -> Result<File, Box<dyn Error>> {
  let file_path = file_path.as_ref();
  fs::create_dir_all(file_path.parent().unwrap())?;
  let file = OpenOptions::new()
    .write(true)
    .create(true)
    .append(append)
    .open(file_path)?;
  Ok(file)
}

fn apply_diff(diff: Patch<str>, destination_file_path: impl AsRef<Path>) {
  let destination_text = read_to_string(destination_file_path.as_ref())
    .expect("failed to read destination file text");
  let destination_text = diffy::apply(&destination_text, &diff)
    .expect("failed to apply diff");
  let mut file = open_writable_file(destination_file_path, false)
    .expect("failed to open destination file");
  file.write_all(destination_text.as_bytes())
    .expect("failed to write to destination file");
}
