use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};

use diffy::Patch;

use crate::stepper::Stepper;

pub enum Modification {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  CreateDiffAndApply(CreateDiffAndApply),
  ApplyDiff(ApplyDiff),
}

impl Modification {
  pub fn apply(&self, stepper: &Stepper) {
    match self {
      Modification::CreateFile(m) => m.apply(stepper),
      Modification::AddToFile(m) => m.apply(stepper),
      Modification::CreateDiffAndApply(m) => m.apply(stepper),
      Modification::ApplyDiff(m) => m.apply(stepper),
    }
  }
}


// Create file

pub struct CreateFile {
  file_path: PathBuf,
}

impl CreateFile {
  pub fn new(file_path: impl Into<PathBuf>) -> Modification {
    let file_path = file_path.into();
    Modification::CreateFile(Self { file_path })
  }
}

impl CreateFile {
  fn apply(&self, stepper: &Stepper) {
    let file_path = stepper.destination_root_directory.join(&self.file_path);
    println!("Creating empty file {}", file_path.display());
    crate::util::open_writable_file(&file_path, true)
      .expect("failed to create empty file");
  }
}


// Add to file

pub struct AddToFile {
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

impl AddToFile {
  fn apply(&self, stepper: &Stepper) {
    let addition_file_path = stepper.source_root_directory.join(&self.addition_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    println!("Appending {} to {}", addition_file_path.display(), destination_file_path.display());

    let addition_text = read_to_string(&addition_file_path)
      .expect("failed to read addition file to string");
    let mut file = crate::util::open_writable_file(&destination_file_path, true)
      .expect("failed to open writable file");
    write!(file, "{}\n\n", addition_text)
      .expect("failed to append to destination file");
  }
}


// Create diff and apply

pub struct CreateDiffAndApply {
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

impl CreateDiffAndApply {
  fn apply(&self, stepper: &Stepper) {
    let original_file_path = stepper.source_root_directory.join(&self.original_file_path);
    let modified_file_path = stepper.source_root_directory.join(&self.modified_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    let diff_output_file_path = stepper.generated_root_directory.join(&self.diff_output_file_path);
    println!("Diffing {} and {}, applying diff to {}, writing diff to {}", original_file_path.display(), modified_file_path.display(), destination_file_path.display(), diff_output_file_path.display());

    let original_text = read_to_string(original_file_path)
      .expect("failed to read original file text");
    let modified_text = read_to_string(modified_file_path)
      .expect("failed to read modified file text");
    let patch = diffy::create_patch(&original_text, &modified_text);
    crate::util::write_to_file(&patch.to_bytes(), diff_output_file_path, false)
      .expect("failed to write to diff output file");

    apply_patch(patch, &destination_file_path);
  }
}

// Apply diff

pub struct ApplyDiff {
  diff_file_path: PathBuf,
  destination_file_path: PathBuf,
}

impl ApplyDiff {
  pub fn new(
    diff_file_path: impl Into<PathBuf>,
    destination_file_path: impl Into<PathBuf>,
  ) -> Modification {
    let diff_file_path = diff_file_path.into();
    let destination_file_path = destination_file_path.into();
    Modification::ApplyDiff(Self { diff_file_path, destination_file_path })
  }
}

impl ApplyDiff {
  fn apply(&self, stepper: &Stepper) {
    let diff_file_path = stepper.source_root_directory.join(&self.diff_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    println!("Applying diff {} to {}", diff_file_path.display(), destination_file_path.display());

    let diff = read_to_string(diff_file_path)
      .expect("failed to read diff to string");
    let patch = diffy::Patch::from_str(&diff)
      .expect("failed to create patch from diff string");
    apply_patch(patch, &destination_file_path);
  }
}

fn apply_patch(patch: Patch<str>, destination_file_path: impl AsRef<Path>) {
  let destination_text = read_to_string(destination_file_path.as_ref())
    .expect("failed to read destination file text");
  let destination_text = diffy::apply(&destination_text, &patch)
    .expect("failed to apply patch");
  crate::util::write_to_file(destination_text.as_bytes(), destination_file_path, false)
    .expect("failed to write to destination file");
}
