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
  pub destination_directory: PathBuf,
  pub generated_root_directory: PathBuf,
  pub last_original_file: HashMap<PathBuf, PathBuf>,
  pub substitutions: Vec<Substitution>,
  pub run_cargo: bool,
  pub cargo_args: Vec<OsString>,
  pub create_outputs: bool,
}

impl Stepper {
  pub fn new<CA: IntoIterator<Item=AO>, AO: AsRef<OsStr>>(
    source_root_directory: impl Into<PathBuf>,
    destination_root_directory: impl Into<PathBuf>,
    destination_directory: impl Into<PathBuf>,
    generated_root_directory: impl Into<PathBuf>,
    run_cargo: bool,
    cargo_args: CA,
    create_outputs: bool,
  ) -> Self {
    Self {
      source_root_directory: source_root_directory.into(),
      destination_root_directory: destination_root_directory.into(),
      destination_directory: destination_directory.into(),
      generated_root_directory: generated_root_directory.into(),
      last_original_file: Default::default(),
      substitutions: Default::default(),
      run_cargo,
      cargo_args: cargo_args.into_iter().map(|a| a.as_ref().to_owned()).collect(),
      create_outputs,
    }
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

  pub fn add_substitution(&mut self, pattern: impl Into<String>, replacement: impl Into<String>) {
    self.substitutions.push(Substitution::new(pattern, replacement));
  }

  pub fn apply_substitutions(&self, text: &mut String) {
    for substitution in &self.substitutions {
      substitution.apply(text);
    }
  }

  pub fn set_cargo_args(&mut self, cargo_args: impl IntoIterator<Item=impl Into<OsString>>) {
    self.cargo_args = cargo_args.into_iter().map(|a| a.into()).collect();
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
  pub create_outputs: bool,
  pub cargo_output: Option<String>,
}

impl Stepper {
  pub fn apply(&mut self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, Some(true))
  }

  pub fn apply_failure(&mut self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, Some(false))
  }

  pub fn apply_may_fail(&mut self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, None)
  }

  fn apply_expect(&mut self, into_modifications: impl IntoModifications, expect_success: Option<bool>) -> Applied {
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

    self.run_cargo_applied(&self.cargo_args, expect_success)
  }

  pub fn run_cargo(&self, cargo_args: impl IntoIterator<Item=impl Into<OsString>> + Clone, expect_success: Option<bool>) -> Option<String> {
    if !self.run_cargo { return None; }

    let run_cargo = RunCargo::new(cargo_args, &self.destination_directory)
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
    Some(cargo_output)
  }

  pub fn run_cargo_applied(&self, cargo_args: impl IntoIterator<Item=impl Into<OsString>> + Clone, expect_success: Option<bool>) -> Applied {
    let cargo_output = self.run_cargo(cargo_args, expect_success);
    Applied { stepper: self, create_outputs: self.create_outputs, cargo_output }
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


// Substitutions

pub struct Substitution {
  pub pattern: String,
  pub replacement: String,
}

impl Substitution {
  pub fn new(
    pattern: impl Into<String>,
    replacement: impl Into<String>,
  ) -> Self {
    Self {
      pattern: pattern.into(),
      replacement: replacement.into(),
    }
  }

  pub fn apply(&self, text: &mut String) {
    *text = text.replace(&self.pattern, &self.replacement);
  }
}
