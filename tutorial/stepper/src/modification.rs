use std::fmt::{Display, Formatter};
use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use diffy::{DiffOptions, Patch};

use crate::stepper::Stepper;
use crate::util::{add_extension, open_writable_file};

#[derive(Clone)]
pub enum Modification {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  CreateDiffAndApply(CreateDiffAndApply),
  ApplyDiff(ApplyDiff),
}

impl Display for Modification {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::CreateFile(m) => m.fmt(f),
      Self::AddToFile(m) => m.fmt(f),
      Self::CreateDiffAndApply(m) => m.fmt(f),
      Self::ApplyDiff(m) => m.fmt(f),
    }
  }
}

impl Modification {
  pub fn resolve(self, stepper: &Stepper) -> anyhow::Result<ModificationResolved> {
    let resolved = match self {
      Self::CreateFile(m) => ModificationResolved::CreateFile(m.resolve(stepper)?),
      Self::AddToFile(m) => ModificationResolved::AddToFile(m.resolve(stepper)?),
      Self::CreateDiffAndApply(m) => ModificationResolved::CreateDiffAndApply(m.resolve(stepper)?),
      Self::ApplyDiff(m) => ModificationResolved::ApplyDiff(m.resolve(stepper)?),
    };
    Ok(resolved)
  }
}

pub enum ModificationResolved {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  CreateDiffAndApply(CreateDiffAndApplyResolved),
  ApplyDiff(ApplyDiff),
}

impl Display for ModificationResolved {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::CreateFile(m) => m.fmt(f),
      Self::AddToFile(m) => m.fmt(f),
      Self::CreateDiffAndApply(m) => m.fmt(f),
      Self::ApplyDiff(m) => m.fmt(f),
    }
  }
}

impl ModificationResolved {
  pub fn apply(&self, stepper: &mut Stepper) -> anyhow::Result<()> {
    match self {
      Self::CreateFile(m) => m.apply(),
      Self::AddToFile(m) => m.apply(stepper),
      Self::CreateDiffAndApply(m) => m.apply(stepper),
      Self::ApplyDiff(m) => m.apply(),
    }
  }
}


// Create file

#[derive(Clone)]
pub struct CreateFile {
  file_path: PathBuf,
}

impl Display for CreateFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Create empty file {}", self.file_path.display())
  }
}

pub fn create(file_path: impl Into<PathBuf>) -> Modification {
  let file_path = file_path.into();
  Modification::CreateFile(CreateFile { file_path })
}

impl CreateFile {
  fn resolve(self, stepper: &Stepper) -> anyhow::Result<Self> {
    let file_path = stepper.destination_root_directory.join(&self.file_path);
    Ok(Self { file_path })
  }

  fn apply(&self) -> anyhow::Result<()> {
    open_writable_file(&self.file_path, true)
      .context("failed to create empty file")?;
    Ok(())
  }
}


// Add to file

#[derive(Clone)]
pub struct AddToFile {
  addition_file_path: PathBuf,
  destination_file_path: PathBuf,
}

impl Display for AddToFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Append {} to {}", self.addition_file_path.display(), self.destination_file_path.display())
  }
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
  fn resolve(self, stepper: &Stepper) -> anyhow::Result<AddToFile> {
    let addition_file_path = stepper.source_root_directory.join(&self.addition_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    Ok(Self { addition_file_path, destination_file_path })
  }

  fn apply(&self, stepper: &mut Stepper) -> anyhow::Result<()> {
    let addition_text = read_to_string(&self.addition_file_path)
      .context("failed to read addition file to string")?;
    let mut file = open_writable_file(&self.destination_file_path, true)
      .context("failed to open writable file")?;
    write!(file, "{}\n\n", addition_text)
      .context("failed to append to destination file")?;

    stepper.last_original_file.insert(self.destination_file_path.clone(), self.addition_file_path.clone());

    Ok(())
  }
}


// Create diff and apply

#[derive(Default, Clone)]
pub struct CreateDiffAndApply {
  original_file_path: Option<PathBuf>,
  modified_file_path: Option<PathBuf>,
  destination_file_path: Option<PathBuf>,
  diff_output_file_path: Option<PathBuf>,
}

impl Display for CreateDiffAndApply {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Diff {:?} and {:?}, apply diff to {:?}, write diff to {:?}", self.original_file_path, self.modified_file_path, self.destination_file_path, self.diff_output_file_path)
  }
}


impl CreateDiffAndApply {
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

  pub fn into_modification(self) -> Modification {
    Modification::CreateDiffAndApply(self)
  }

  pub fn resolve(self, stepper: &Stepper) -> anyhow::Result<CreateDiffAndApplyResolved> {
    let relative_modified_file_path = self.modified_file_path
      .context("did not set modified file path")?;
    let modified_file_path = stepper.source_root_directory.join(&relative_modified_file_path);
    let destination_file_path = self.destination_file_path
      .context("did not set destination file path")?;
    let destination_file_path = stepper.destination_root_directory.join(&destination_file_path);
    let diff_output_file_path = if let Some(diff_output_file_path) = self.diff_output_file_path {
      diff_output_file_path
    } else {
      let mut diff_output_file_path = relative_modified_file_path.clone();
      add_extension(&mut diff_output_file_path, "diff");
      diff_output_file_path
    };
    let diff_output_file_path = stepper.generated_root_directory.join(&diff_output_file_path);

    let original_file_path = if let Some(original_file_path) = &self.original_file_path {
      stepper.source_root_directory.join(original_file_path)
    } else {
      stepper.last_original_file.get(&destination_file_path)
        .context("failed to get last original file path")?.clone()
    };

    Ok(CreateDiffAndApplyResolved {
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
  CreateDiffAndApply::default()
    .modified(modified_file_path)
    .destination(destination_file_path)
    .into_modification()
}

pub fn create_diff_builder(
  modified_file_path: impl Into<PathBuf>,
  destination_file_path: impl Into<PathBuf>,
) -> CreateDiffAndApply {
  CreateDiffAndApply::default()
    .modified(modified_file_path)
    .destination(destination_file_path)
}

pub struct CreateDiffAndApplyResolved {
  original_file_path: PathBuf,
  modified_file_path: PathBuf,
  destination_file_path: PathBuf,
  diff_output_file_path: PathBuf,
}

impl Display for CreateDiffAndApplyResolved {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Diff {} and {}, apply diff to {}, write diff to {}", self.original_file_path.display(), self.modified_file_path.display(), self.destination_file_path.display(), self.diff_output_file_path.display())
  }
}

impl CreateDiffAndApplyResolved {
  fn apply(&self, stepper: &mut Stepper) -> anyhow::Result<()> {
    let original_text = read_to_string(&self.original_file_path)
      .context("failed to read original file text")?;
    let modified_text = read_to_string(&self.modified_file_path)
      .context("failed to read modified file text")?;
    let mut diff_options = DiffOptions::default();
    diff_options.set_context_len(5);
    let patch = diff_options.create_patch(&original_text, &modified_text);
    crate::util::write_to_file(&patch.to_bytes(), &self.diff_output_file_path, false)
      .context("failed to write to diff output file")?;

    apply_patch(patch, &self.destination_file_path)?;

    stepper.last_original_file.insert(self.destination_file_path.clone(), self.modified_file_path.clone());

    Ok(())
  }
}


// Apply diff

#[derive(Clone)]
pub struct ApplyDiff {
  diff_file_path: PathBuf,
  destination_file_path: PathBuf,
}

impl Display for ApplyDiff {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Apply diff {} to {}", self.diff_file_path.display(), self.destination_file_path.display())
  }
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
  fn resolve(self, stepper: &Stepper) -> anyhow::Result<ApplyDiff> {
    let diff_file_path = stepper.source_root_directory.join(&self.diff_file_path);
    let destination_file_path = stepper.destination_root_directory.join(&self.destination_file_path);
    Ok(Self { diff_file_path, destination_file_path })
  }

  fn apply(&self) -> anyhow::Result<()> {
    let diff = read_to_string(&self.diff_file_path)
      .context("failed to read diff to string")?;
    let patch = diffy::Patch::from_str(&diff)
      .context("failed to create patch from diff string")?;
    apply_patch(patch, &self.destination_file_path)?;

    Ok(())
  }
}

fn apply_patch(patch: Patch<str>, destination_file_path: impl AsRef<Path>) -> anyhow::Result<()> {
  let destination_text = read_to_string(destination_file_path.as_ref())
    .context("failed to read destination file text")?;
  let destination_text = diffy::apply(&destination_text, &patch)
    .context("failed to apply patch")?;
  crate::util::write_to_file(destination_text.as_bytes(), destination_file_path, false)
    .context("failed to write to destination file")?;
  Ok(())
}
