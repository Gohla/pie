use std::error::Error;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::fs::{File, OpenOptions, read_to_string};
use std::io::{BufReader, Read, Write};
use std::path::{Path, PathBuf};

use diffy::Patch;

fn main() {
  let temp_directory = tempfile::tempdir().expect("failed to create temporary directory");
  let mut stepper = Stepper::new(
    "../src/",
    temp_directory.path().join("src"),
    "../src/diff",
    "../src/out",
    ["build"],
  );
  stepper.push_chapter("api");
  stepper.apply("0.txt", [
    AddToFile::new("0_Cargo.toml", "../Cargo.toml"),
    AddToFile::new("0_api.rs", "lib.rs"),
  ]);
  stepper.apply("2.txt", [
    CreateDiffAndApply::new("0_api.rs", "1_context_module.rs", "lib.rs", "1_context_module.rs.diff"),
    AddToFile::new("2_non_incremental_module.rs", "context/mod.rs"),
    CreateFile::new("context/non_incremental.rs")
  ]);
  stepper.apply("3.txt", AddToFile::new("3_non_incremental_context.rs", "context/non_incremental.rs"));
  stepper.set_cargo_args(["test"]);
  stepper.apply("4.txt", AddToFile::new("4_test_1.rs", "context/non_incremental.rs"));
  stepper.apply("5.txt", CreateDiffAndApply::new("4_test_1.rs", "5_test_2.rs", "context/non_incremental.rs", "5_test_2.rs.diff"));
  stepper.pop_chapter();
}

struct Stepper {
  source_root_directory: PathBuf,
  destination_root_directory: PathBuf,
  diff_output_root_directory: PathBuf,
  cargo_output_root_directory: PathBuf,
  cargo_args: Vec<OsString>,
}

impl Stepper {
  pub fn new<CA: IntoIterator<Item=AO>, AO: AsRef<OsStr>>(
    source_root_directory: impl Into<PathBuf>,
    destination_root_directory: impl Into<PathBuf>,
    diff_output_root_directory: impl Into<PathBuf>,
    cargo_output_root_directory: impl Into<PathBuf>,
    cargo_args: CA,
  ) -> Self {
    let source_root_directory = source_root_directory.into();
    let destination_root_directory = destination_root_directory.into();
    let diff_output_root_directory = diff_output_root_directory.into();
    let cargo_output_root_directory = cargo_output_root_directory.into();
    let cargo_args = cargo_args.into_iter().map(|ao| ao.as_ref().to_owned()).collect();
    Self { source_root_directory, destination_root_directory, diff_output_root_directory, cargo_output_root_directory, cargo_args }
  }

  pub fn push_chapter(&mut self, path: impl AsRef<Path>) {
    self.source_root_directory.push(&path);
    self.diff_output_root_directory.push(&path);
    self.cargo_output_root_directory.push(&path);
  }

  pub fn pop_chapter(&mut self) {
    self.source_root_directory.pop();
    self.diff_output_root_directory.pop();
    self.cargo_output_root_directory.pop();
  }

  pub fn set_cargo_args<CA: IntoIterator<Item=AO>, AO: AsRef<OsStr>>(&mut self, cargo_args: CA) {
    self.cargo_args = cargo_args.into_iter().map(|ao| ao.as_ref().to_owned()).collect();
  }
}


// Apply modifications

enum Modification {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  CreateDiffAndApply(CreateDiffAndApply),
}

trait IntoModifications {
  type Output: IntoIterator<Item=Modification>;
  fn into(self) -> Self::Output;
}

impl<T: IntoIterator<Item=Modification>> IntoModifications for T {
  type Output = T;
  fn into(self) -> Self::Output { self }
}

impl IntoModifications for Modification {
  type Output = [Modification; 1];
  fn into(self) -> Self::Output { [self] }
}

impl Stepper {
  pub fn apply(&self, cargo_output_file_path: impl AsRef<Path>, into_modifications: impl IntoModifications) {
    for modification in into_modifications.into() {
      match modification {
        Modification::CreateFile(create_file) => self.create_file(create_file),
        Modification::AddToFile(add_to_file) => self.add_to_file(add_to_file),
        Modification::CreateDiffAndApply(create_diff_and_apply) => self.create_diff_and_apply(create_diff_and_apply),
      }
    }
    let cargo_output_file_path = self.cargo_output_root_directory.join(cargo_output_file_path);
    self.run_cargo(cargo_output_file_path);
  }

  fn run_cargo(&self, output_file_path: impl AsRef<Path>) {
    let cmd = duct::cmd("cargo", &self.cargo_args)
      .dir(&self.destination_root_directory);
    let mut cmd_joined = vec!["cargo".to_string()];
    cmd_joined.extend(self.cargo_args.iter().map(|oss|oss.clone().into_string().expect("failed to convert cmd to string")));
    let cmd_joined = cmd_joined.join(" ");
    println!("> {}", cmd_joined);
    let reader = cmd.stderr_to_stdout().reader()
      .expect("failed to create stdout/stderr reader");
    let mut reader = BufReader::new(reader);
    let mut output = String::new();
    reader.read_to_string(&mut output)
      .expect("failed to read stdout/stderr to string");
    print!("{}", output);
    let mut output_file = open_writable_file(output_file_path, false)
      .expect("failed to open output file");
    writeln!(output_file, "> {}", cmd_joined)
      .expect("failed to write command to output file");
    output_file.write_all(output.as_bytes())
      .expect("failed to write stdout/stderr to output file");
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
    .truncate(!append)
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
