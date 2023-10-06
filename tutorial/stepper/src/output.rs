use std::fs;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use termtree::Tree;
use walkdir::WalkDir;
use zip::write::FileOptions;

use crate::stepper::Applied;
use crate::util::{is_hidden, open_writable_file};

pub enum Output {
  CargoOutput(CargoOutput),
  DirectoryStructure(DirectoryStructure),
  SourceArchive(SourceArchive)
}

impl Output {
  pub fn apply(&self, applied: &Applied) -> anyhow::Result<()> {
    match self {
      Output::CargoOutput(o) => o.apply(applied),
      Output::DirectoryStructure(o) => o.apply(applied),
      Output::SourceArchive(o) => o.apply(applied),
    }
  }
}


// Cargo output

pub struct CargoOutput {
  output_file_path: PathBuf,
  modify_fn: Option<Box<dyn Fn(String) -> String>>,
}

impl CargoOutput {
  pub fn new(output_file_path: impl Into<PathBuf>) -> Output {
    let output_file_path = output_file_path.into();
    Output::CargoOutput(Self { output_file_path, modify_fn: None })
  }

  pub fn with_modify_fn(output_file_path: impl Into<PathBuf>, modify_fn: impl Fn(String) -> String + 'static) -> Output {
    let output_file_path = output_file_path.into();
    Output::CargoOutput(Self { output_file_path, modify_fn: Some(Box::new(modify_fn)) })
  }
}

impl CargoOutput {
  fn apply(&self, applied: &Applied) -> anyhow::Result<()> {
    if !applied.create_outputs { return Ok(()); }
    let Some(cargo_output) = &applied.cargo_output else {
      return Ok(());
    };

    let output_file_path = applied.stepper.generated_root_directory.join(&self.output_file_path);
    let mut cargo_output = if let Some(str) = applied.stepper.destination_root_directory.to_str() {
      cargo_output.replace(str, "")
    } else {
      cargo_output.clone()
    };

    if let Some(modify_fn) = &self.modify_fn {
      cargo_output = modify_fn(cargo_output);
    }

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
    if !applied.create_outputs { return Ok(()); }

    let destination_directory_path = applied.stepper.destination_directory.join(&self.destination_directory_path);
    let tree = Self::directory_tree(&destination_directory_path)
      .context("failed to create directory structure")?;

    let output_file_path = applied.stepper.generated_root_directory.join(&self.output_file_path);
    let mut file = open_writable_file(&output_file_path, false)
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
        if dir.is_dir() && !is_hidden(&entry.file_name()) {
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


// Source archive

pub struct SourceArchive {
  zip_file_path: PathBuf,
}

impl SourceArchive {
  pub fn new(zip_file_path: impl Into<PathBuf>) -> Output {
    let zip_file_path = zip_file_path.into();
    Output::SourceArchive(Self { zip_file_path })
  }
}

impl SourceArchive {
  fn apply(&self, applied: &Applied) -> anyhow::Result<()> {
    if !applied.create_outputs { return Ok(()); }

    let zip_file_path = applied.stepper.generated_root_directory.join(&self.zip_file_path);
    let zip_file = open_writable_file(&zip_file_path, false)
      .context("failed to open writable file")?;
    let mut zip = zip::ZipWriter::new(zip_file);
    let zip_file_options = FileOptions::default();

    let mut buffer = Vec::new();
    let source_directory = &applied.stepper.destination_root_directory;
    let walker = WalkDir::new(source_directory).into_iter();
    for entry in walker.filter_entry(|e| !is_hidden(e.file_name())) {
      let entry = entry?;
      let path = entry.path();
      let name = path.strip_prefix(source_directory)?.to_string_lossy();
      if entry.metadata()?.is_file() {
        zip.start_file(name, zip_file_options)?;
        let mut file = File::open(path)?;
        file.read_to_end(&mut buffer)?;
        zip.write_all(&buffer)?;
        buffer.clear();
      } else if !name.is_empty() {
        zip.add_directory(name, zip_file_options)?;
      }
    }
    zip.finish()?;

    Ok(())
  }
}
