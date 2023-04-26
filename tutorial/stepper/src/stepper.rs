use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use tracing::{debug, error, info};

use crate::cargo::RunCargo;
use crate::modification::Modification;
use crate::output::Output;

pub struct Stepper {
  pub source_root_directory: PathBuf,
  pub destination_root_directory: PathBuf,
  pub generated_root_directory: PathBuf,
  pub last_original_file: HashMap<PathBuf, PathBuf>,
  pub cargo_args: Vec<OsString>,
}

impl Stepper {
  pub fn new<CA: IntoIterator<Item=AO>, AO: AsRef<OsStr>>(
    source_root_directory: impl Into<PathBuf>,
    destination_root_directory: impl Into<PathBuf>,
    generated_root_directory: impl Into<PathBuf>,
    cargo_args: CA,
  ) -> Self {
    let source_root_directory = source_root_directory.into();
    let destination_root_directory = destination_root_directory.into();
    let generated_root_directory = generated_root_directory.into();
    let last_original_file = HashMap::new();
    let cargo_args = cargo_args.into_iter().map(|ao| ao.as_ref().to_owned()).collect();
    Self { source_root_directory, destination_root_directory, generated_root_directory, last_original_file, cargo_args }
  }

  pub fn push_path(&mut self, path: impl AsRef<Path>) {
    self.source_root_directory.push(&path);
    self.generated_root_directory.push(&path);
  }

  pub fn pop_path(&mut self) {
    self.source_root_directory.pop();
    self.generated_root_directory.pop();
  }

  pub fn with_path<R>(&mut self, path: impl AsRef<Path>, func: impl FnOnce(&mut Self) -> R) -> R {
    self.push_path(path);
    let result = func(self);
    self.pop_path();
    result
  }

  pub fn set_cargo_args<CA: IntoIterator<Item=AO>, AO: AsRef<OsStr>>(&mut self, cargo_args: CA) {
    self.cargo_args = cargo_args.into_iter().map(|ao| ao.as_ref().to_owned()).collect();
  }
}

// Apply modifications

pub trait IntoModifications {
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

pub struct Applied<'a> {
  pub stepper: &'a Stepper,
  pub cargo_output: String,
}

impl Stepper {
  pub fn apply(&mut self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, true)
  }

  pub fn apply_failure(&mut self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, false)
  }

  fn apply_expect(&mut self, into_modifications: impl IntoModifications, expect_success: bool) -> Applied {
    for modification in into_modifications.into() {
      debug!("Resolve: {}", modification);
      let resolved = modification.clone().resolve(&self);
      if let Err(e) = resolved {
        error!("Failed to resolve modification:\n  {}\nError:\n  {}", modification, e);
        panic!("Failed to resolve modification; stopping");
      }
      let resolved = resolved.unwrap();

      info!("Apply: {}", resolved);
      if let Err(e) = resolved.apply(self) {
        error!("Failed to apply modification:\n  {}\nError:\n  {}", modification, e);
        panic!("Failed to apply modification; stopping");
      }
    }

    let run_cargo = RunCargo::new(&self)
      .expect("failed to create run cargo command");
    info!("{}", run_cargo);

    let (cargo_output, valid) = run_cargo.run(expect_success)
      .expect("failed to run cargo command or failed to get its output");
    if !valid {
      error!("Cargo run did not result in expected outcome. Command:\n{}\nOutput:\n{}", run_cargo, cargo_output);
      panic!("Cargo run did not result in expected outcome; stopping");
    } else {
      info!("{}", cargo_output);
    }
    Applied { stepper: self, cargo_output }
  }
}


// Create outputs

pub trait IntoOutputs {
  type Output: IntoIterator<Item=Output>;
  fn into(self) -> Self::Output;
}

impl<T: IntoIterator<Item=Output>> IntoOutputs for T {
  type Output = T;
  fn into(self) -> Self::Output { self }
}

impl IntoOutputs for Output {
  type Output = [Output; 1];
  fn into(self) -> Self::Output { [self] }
}

impl<'a> Applied<'a> {
  pub fn output(&self, into_outputs: impl IntoOutputs) {
    for output in into_outputs.into() {
      output.apply(self)
        .expect("failed to apply output");
    }
  }
}
