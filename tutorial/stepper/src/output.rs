use std::ffi::OsString;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

use anyhow::Context;
use termtree::Tree;

use crate::stepper::Applied;

pub enum Output {
  CargoOutput(CargoOutput),
  DirectoryStructure(DirectoryStructure),
}

impl Output {
  pub fn apply(&self, applied: &Applied) -> anyhow::Result<()> {
    match self {
      Output::CargoOutput(o) => o.apply(applied),
      Output::DirectoryStructure(o) => o.apply(applied),
    }
  }
}


// Cargo output

pub struct CargoOutput {
  output_file_path: PathBuf,
}

impl CargoOutput {
  pub fn new(output_file_path: impl Into<PathBuf>) -> Output {
    let output_file_path = output_file_path.into();
    Output::CargoOutput(Self { output_file_path })
  }
}

impl CargoOutput {
  fn apply(&self, applied: &Applied) -> anyhow::Result<()> {
    let output_file_path = applied.stepper.generated_root_directory.join(&self.output_file_path);
    let cargo_output = if let Some(str) = applied.stepper.destination_root_directory.to_str() {
      applied.cargo_output.replace(str, "")
    } else {
      applied.cargo_output.clone()
    };
    crate::util::write_to_file(cargo_output.as_bytes(), output_file_path, false)
      .context("failed to write cargo output to file")?;
    Ok(())
  }
}


// Directory structure

pub struct DirectoryStructure {
  destination_directory_path: PathBuf,
  output_file_path: PathBuf,
}

impl DirectoryStructure {
  pub fn new(
    destination_directory_path: impl Into<PathBuf>,
    output_file_path: impl Into<PathBuf>
  ) -> Output {
    let destination_directory_path = destination_directory_path.into();
    let output_file_path = output_file_path.into();
    Output::DirectoryStructure(Self { destination_directory_path, output_file_path })
  }
}

impl DirectoryStructure {
  fn apply(&self, applied: &Applied) -> anyhow::Result<()> {
    let destination_directory_path = applied.stepper.destination_directory.join(&self.destination_directory_path);
    let tree = Self::directory_tree(&destination_directory_path)
      .context("failed to create directory structure")?;

    let output_file_path = applied.stepper.generated_root_directory.join(&self.output_file_path);
    let mut file = crate::util::open_writable_file(&output_file_path, false)
      .context("failed to open writable file")?;
    write!(file, "{}", tree)
      .context("failed to write directory structure to file")?;
    Ok(())
  }

  fn directory_tree(path: impl AsRef<Path>) -> Result<Tree<String>, std::io::Error> {
    fn label(p: impl AsRef<Path>) -> String {
      p.as_ref().file_name().unwrap().to_str().unwrap().to_owned()
    }
    let result = fs::read_dir(&path)?.filter_map(|e| e.ok()).fold(
      Tree::new(label(path.as_ref().canonicalize()?)),
      |mut root, entry| {
        let dir = entry.metadata().unwrap();
        if dir.is_dir() && entry.file_name() != OsString::from_str("target").unwrap() {
          root.push(Self::directory_tree(entry.path()).unwrap());
        } else {
          root.push(Tree::new(label(entry.path())));
        }
        root
      },
    );
    Ok(result)
  }
}
