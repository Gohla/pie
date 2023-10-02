use std::fmt::{Display, Formatter};
use std::fs::read_to_string;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use path_slash::PathExt;
use similar::TextDiff;

use crate::stepper::Stepper;
use crate::util::{add_extension, open_writable_file, write_to_file};

#[derive(Clone)]
pub enum Modification {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  InsertIntoFile(InsertIntoFile),
  CreateDiffAndApply(CreateDiffAndApply),
  ApplyDiff(ApplyDiff),
}

impl Display for Modification {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::CreateFile(m) => m.fmt(f),
      Self::AddToFile(m) => m.fmt(f),
      Self::InsertIntoFile(m) => m.fmt(f),
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
      Self::InsertIntoFile(m) => ModificationResolved::InsertIntoFile(m.resolve(stepper)?),
      Self::CreateDiffAndApply(m) => ModificationResolved::CreateDiffAndApply(m.resolve(stepper)?),
      Self::ApplyDiff(m) => ModificationResolved::ApplyDiff(m.resolve(stepper)?),
    };
    Ok(resolved)
  }
}

pub enum ModificationResolved {
  CreateFile(CreateFile),
  AddToFile(AddToFile),
  InsertIntoFile(InsertIntoFile),
  CreateDiffAndApply(CreateDiffAndApplyResolved),
  ApplyDiff(ApplyDiff),
}

impl Display for ModificationResolved {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::CreateFile(m) => m.fmt(f),
      Self::AddToFile(m) => m.fmt(f),
      Self::InsertIntoFile(m) => m.fmt(f),
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
      Self::InsertIntoFile(m) => m.apply(stepper),
      Self::CreateDiffAndApply(m) => m.apply(stepper),
      Self::ApplyDiff(m) => m.apply(stepper),
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
    let file_path = stepper.destination_directory.join(&self.file_path);
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
    let destination_file_path = stepper.destination_directory.join(&self.destination_file_path);
    Ok(Self { addition_file_path, destination_file_path })
  }

  fn apply(&self, stepper: &mut Stepper) -> anyhow::Result<()> {
    let mut addition_text = read_to_string(&self.addition_file_path)
      .context("failed to read addition file to string")?;
    stepper.apply_substitutions(&mut addition_text);
    let mut file = open_writable_file(&self.destination_file_path, true)
      .context("failed to open writable file")?;
    write!(file, "{}", addition_text)
      .context("failed to append to destination file")?;

    stepper.last_original_file.insert(self.destination_file_path.clone(), self.addition_file_path.clone());

    Ok(())
  }
}


// Insert into to file

#[derive(Clone, Debug)]
pub enum InsertionPlace {
  BeforeLine(usize),
  BeforeLastMatchOf(&'static str),
}

impl From<usize> for InsertionPlace {
  fn from(value: usize) -> Self { Self::BeforeLine(value) }
}

impl From<&'static str> for InsertionPlace {
  fn from(value: &'static str) -> Self { Self::BeforeLastMatchOf(value) }
}

#[derive(Clone)]
pub struct InsertIntoFile {
  insertion_file_path: PathBuf,
  insertion_place: InsertionPlace,
  destination_file_path: PathBuf,
}

pub fn insert(
  insertion_file_path: impl Into<PathBuf>,
  insertion_place: impl Into<InsertionPlace>,
  destination_file_path: impl Into<PathBuf>,
) -> Modification {
  let insertion_file_path = insertion_file_path.into();
  let insertion_place = insertion_place.into();
  let destination_file_path = destination_file_path.into();
  Modification::InsertIntoFile(InsertIntoFile { insertion_file_path, insertion_place, destination_file_path })
}

impl Display for InsertIntoFile {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "Insert {} into {} at {:?}", self.insertion_file_path.display(), self.destination_file_path.display(), self.insertion_place)
  }
}

impl InsertIntoFile {
  fn resolve(self, stepper: &Stepper) -> anyhow::Result<InsertIntoFile> {
    let insertion_file_path = stepper.source_root_directory.join(&self.insertion_file_path);
    let insertion_place = self.insertion_place;
    let destination_file_path = stepper.destination_directory.join(&self.destination_file_path);
    Ok(Self { insertion_file_path, insertion_place, destination_file_path })
  }

  fn apply(&self, stepper: &Stepper) -> anyhow::Result<()> {
    let insertion_text = read_to_string(&self.insertion_file_path)
      .context("failed to read insertion file to string")?;
    let destination_text = read_to_string(&self.destination_file_path)
      .context("failed to read destination file to string")?;
    let mut new_text = match &self.insertion_place {
      InsertionPlace::BeforeLine(line) => {
        let between_lines: Vec<_> = insertion_text.lines().collect();
        let destination_lines: Vec<_> = destination_text.lines().collect();
        let (before, after) = destination_lines.split_at(line - 2); // -1 to make it 0 based, -1 to get the line to insert after.
        let mut new_lines = Vec::with_capacity(before.len() + between_lines.len() + after.len());
        new_lines.extend_from_slice(before);
        new_lines.extend_from_slice(&between_lines);
        new_lines.extend_from_slice(after);
        new_lines.join("\n")
      }
      InsertionPlace::BeforeLastMatchOf(pattern) => {
        let Some(index) = destination_text.rfind(pattern) else {
          bail!("failed to insert before last match of {}, that pattern was not found", pattern);
        };
        let (before, after) = destination_text.split_at(index);
        format!("{}{}{}", before, insertion_text, after)
      }
    };
    stepper.apply_substitutions(&mut new_text);
    write_to_file(new_text.as_bytes(), &self.destination_file_path, false)
      .context("failed to write to destination file")?;

    Ok(())
  }
}


// Create diff and apply

#[derive(Default, Clone)]
pub struct CreateDiffAndApply {
  original_file_path: Option<PathBuf>,
  use_destination_file_as_original_file_if_unset: bool,
  modified_file_path: Option<PathBuf>,
  destination_file_path: Option<PathBuf>,
  diff_output_file_path: Option<PathBuf>,
  context_length: Option<usize>,
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
  pub fn use_destination_file_as_original_file_if_unset(mut self, use_destination_file_as_original_file_if_unset: bool) -> Self {
    self.use_destination_file_as_original_file_if_unset = use_destination_file_as_original_file_if_unset;
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
  pub fn context_length(mut self, context_length: usize) -> Self {
    self.context_length = Some(context_length);
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
    let destination_file_path = stepper.destination_directory.join(&destination_file_path);
    let diff_output_file_path = if let Some(diff_output_file_path) = self.diff_output_file_path {
      diff_output_file_path
    } else {
      let mut diff_output_file_path = relative_modified_file_path.clone();
      add_extension(&mut diff_output_file_path, "diff");
      diff_output_file_path
    };
    let diff_output_file_path = stepper.generated_root_directory.join(&diff_output_file_path);
    let context_length = self.context_length.unwrap_or(3);

    let original_file_path = if let Some(original_file_path) = &self.original_file_path {
      stepper.source_root_directory.join(original_file_path)
    } else if self.use_destination_file_as_original_file_if_unset {
      destination_file_path.clone()
    } else {
      stepper.last_original_file.get(&destination_file_path)
        .context("failed to get last original file path")?.clone()
    };

    Ok(CreateDiffAndApplyResolved {
      original_file_path,
      modified_file_path,
      destination_file_path,
      diff_output_file_path,
      context_length,
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

pub fn create_diff_from_destination_file(
  modified_file_path: impl Into<PathBuf>,
  destination_file_path: impl Into<PathBuf>,
) -> Modification {
  CreateDiffAndApply::default()
    .modified(modified_file_path)
    .destination(destination_file_path)
    .use_destination_file_as_original_file_if_unset(true)
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
  context_length: usize,
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
    let original_text = normalize_to_unix_line_endings(original_text); // Normalize to Unix line endings for diffy.
    let modified_text = read_to_string(&self.modified_file_path)
      .context("failed to read modified file text")?;
    let mut modified_text = normalize_to_unix_line_endings(modified_text);
    stepper.apply_substitutions(&mut modified_text);

    let destination_file_path = dunce::canonicalize(&self.destination_file_path)
      .with_context(|| format!("failed to canonicalize destination file path '{}' for unified diff header", self.destination_file_path.display()))?;
    let destination_root_directory = dunce::canonicalize(&stepper.destination_root_directory)
      .with_context(|| format!("failed to canonicalize destination root directory '{}' for unified diff header", stepper.destination_root_directory.display()))?;
    let header_file_name = destination_file_path.strip_prefix(&destination_root_directory)
      .with_context(|| format!("failed to get relative file name for unified diff header by stripping prefix '{}' from: {}", destination_root_directory.display(), destination_file_path.display()))?
      .to_slash_lossy();

    let diff = TextDiff::from_lines(&original_text, &modified_text);
    let unified_diff = diff.unified_diff()
      .header(&header_file_name, &header_file_name)
      .context_radius(self.context_length)
      .to_string();
    write_to_file(unified_diff.as_bytes(), &self.diff_output_file_path, false)
      .context("failed to write to diff output file")?;

    let patch = diffy::Patch::from_str(&unified_diff)
      .context("failed to parse unified diff")?;
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
    let destination_file_path = stepper.destination_directory.join(&self.destination_file_path);
    Ok(Self { diff_file_path, destination_file_path })
  }

  fn apply(&self, stepper: &Stepper) -> anyhow::Result<()> {
    let diff = read_to_string(&self.diff_file_path)
      .context("failed to read diff to string")?;
    let mut diff = normalize_to_unix_line_endings(diff); // Normalize to Unix line endings for diffy.
    stepper.apply_substitutions(&mut diff);
    let patch = diffy::Patch::from_str(&diff)
      .context("failed to parse unified diff")?;
    apply_patch(patch, &self.destination_file_path)?;

    Ok(())
  }
}

fn normalize_to_unix_line_endings(str: impl AsRef<str>) -> String {
  str.as_ref().replace("\r\n", "\n")
}

fn apply_patch(patch: diffy::Patch<str>, file_path: impl AsRef<Path>) -> anyhow::Result<()> {
  let text = read_to_string(file_path.as_ref())
    .context("failed to read file text")?;
  let text = normalize_to_unix_line_endings(text); // Normalize to Unix line endings for diffy.
  let text = diffy::apply(&text, &patch)
    .context("failed to apply patch")?;
  write_to_file(text.as_bytes(), file_path, false)
    .context("failed to write patched text to file")?;
  Ok(())
}
