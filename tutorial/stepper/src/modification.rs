use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};

use diffy::{DiffOptions, Patch};

use crate::stepper::Stepper;
use crate::util::add_extension;

pub enum Modification {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  CreateDiffAndApply(CreateDiffAndApply),
  ApplyDiff(ApplyDiff),
}

impl Modification {
  pub fn apply(&self, stepper: &mut Stepper) {
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

pub fn create(file_path: impl Into<PathBuf>) -> Modification {
  let file_path = file_path.into();
  Modification::CreateFile(CreateFile { file_path })
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

pub fn add(
  addition_file_path: impl Into<PathBuf>,
  destination_file_path: impl Into<PathBuf>,
) -> Modification {
  let addition_file_path = addition_file_path.into();
  let destination_file_path = destination_file_path.into();
  Modification::AddToFile(AddToFile { addition_file_path, destination_file_path })
}

impl AddToFile {
  fn apply(&self, stepper: &mut Stepper) {
    let addition_file_path = stepper.source_root_directory.join(&self.addition_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    println!("Appending {} to {}", addition_file_path.display(), destination_file_path.display());

    let addition_text = read_to_string(&addition_file_path)
      .expect("failed to read addition file to string");
    let mut file = crate::util::open_writable_file(&destination_file_path, true)
      .expect("failed to open writable file");
    write!(file, "{}\n\n", addition_text)
      .expect("failed to append to destination file");

    stepper.last_original_file.insert(self.destination_file_path.clone(), addition_file_path);
  }
}


// Create diff and apply

#[derive(Default)]
pub struct CreateDiffAndApplyBuilder {
  original_file_path: Option<PathBuf>,
  modified_file_path: Option<PathBuf>,
  destination_file_path: Option<PathBuf>,
  diff_output_file_path: Option<PathBuf>,
}

impl CreateDiffAndApplyBuilder {
  pub fn original(mut self, path: impl Into<PathBuf>) -> Self {
    self.original_file_path = Some(path.into());
    self
  }
  pub fn modified(mut self, path: impl Into<PathBuf>) -> Self {
    self.modified_file_path = Some(path.into());
    self
  }
  pub fn destination(mut self, path: impl Into<PathBuf>) -> Self {
    self.destination_file_path = Some(path.into());
    self
  }
  #[allow(dead_code)]
  pub fn diff_output(mut self, path: impl Into<PathBuf>) -> Self {
    self.diff_output_file_path = Some(path.into());
    self
  }

  pub fn build(self) -> Modification {
    let original_file_path = self.original_file_path;
    let modified_file_path = self.modified_file_path.expect("did not set modified file path");
    let destination_file_path = self.destination_file_path.expect("did not set destination file path");
    let diff_output_file_path = if let Some(diff_output_file_path) = self.diff_output_file_path {
      diff_output_file_path
    } else {
      let mut diff_output_file_path = modified_file_path.clone();
      add_extension(&mut diff_output_file_path, "diff");
      diff_output_file_path
    };
    Modification::CreateDiffAndApply(CreateDiffAndApply {
      original_file_path,
      modified_file_path,
      destination_file_path,
      diff_output_file_path,
    })
  }
}

pub fn create_diff(
  modified_file_path: impl Into<PathBuf>,
  destination_file_path: impl Into<PathBuf>,
) -> Modification {
  CreateDiffAndApplyBuilder::default()
    .modified(modified_file_path)
    .destination(destination_file_path)
    .build()
}

pub fn create_diff_builder(
  modified_file_path: impl Into<PathBuf>,
  destination_file_path: impl Into<PathBuf>,
) -> CreateDiffAndApplyBuilder {
  CreateDiffAndApplyBuilder::default()
    .modified(modified_file_path)
    .destination(destination_file_path)
}

pub struct CreateDiffAndApply {
  original_file_path: Option<PathBuf>,
  modified_file_path: PathBuf,
  destination_file_path: PathBuf,
  diff_output_file_path: PathBuf,
}

impl CreateDiffAndApply {
  fn apply(&self, stepper: &mut Stepper) {
    let original_file_path = if let Some(original_file_path) = &self.original_file_path {
      stepper.source_root_directory.join(original_file_path)
    } else {
      stepper.last_original_file.get(&self.destination_file_path)
        .expect("failed to get last original file path").clone()
    };
    let modified_file_path = stepper.source_root_directory.join(&self.modified_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    let diff_output_file_path = stepper.generated_root_directory.join(&self.diff_output_file_path);
    println!("Diffing {} and {}, applying diff to {}, writing diff to {}", original_file_path.display(), modified_file_path.display(), destination_file_path.display(), diff_output_file_path.display());

    let original_text = read_to_string(&original_file_path)
      .expect("failed to read original file text");
    let modified_text = read_to_string(&modified_file_path)
      .expect("failed to read modified file text");
    let mut diff_options = DiffOptions::default();
    diff_options.set_context_len(5);
    let patch = diff_options.create_patch(&original_text, &modified_text);
    crate::util::write_to_file(&patch.to_bytes(), diff_output_file_path, false)
      .expect("failed to write to diff output file");

    apply_patch(patch, &destination_file_path);

    stepper.last_original_file.insert(self.destination_file_path.clone(), modified_file_path);
  }
}

// Apply diff

pub struct ApplyDiff {
  diff_file_path: PathBuf,
  destination_file_path: PathBuf,
}

pub fn apply_diff(
  diff_file_path: impl Into<PathBuf>,
  destination_file_path: impl Into<PathBuf>,
) -> Modification {
  let diff_file_path = diff_file_path.into();
  let destination_file_path = destination_file_path.into();
  Modification::ApplyDiff(ApplyDiff { diff_file_path, destination_file_path })
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
