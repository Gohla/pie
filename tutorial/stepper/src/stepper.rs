use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use crate::modification::Modification;
use crate::output::Output;

pub struct Stepper {
  pub source_root_directory: PathBuf,
  pub destination_root_directory: PathBuf,
  pub generated_root_directory: PathBuf,
  cargo_args: Vec<OsString>,
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
    let cargo_args = cargo_args.into_iter().map(|ao| ao.as_ref().to_owned()).collect();
    Self { source_root_directory, destination_root_directory, generated_root_directory, cargo_args }
  }

  pub fn push_chapter(&mut self, path: impl AsRef<Path>) {
    self.source_root_directory.push(&path);
    self.generated_root_directory.push(&path);
  }

  pub fn pop_chapter(&mut self) {
    self.source_root_directory.pop();
    self.generated_root_directory.pop();
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
  pub fn apply(&self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, true)
  }

  pub fn apply_failure(&self, into_modifications: impl IntoModifications) -> Applied {
    self.apply_expect(into_modifications, false)
  }

  fn apply_expect(&self, into_modifications: impl IntoModifications, expect_success: bool) -> Applied {
    for modification in into_modifications.into() {
      modification.apply(self);
    }
    let cargo_output = self.run_cargo(expect_success);
    Applied { stepper: self, cargo_output }
  }

  fn run_cargo(&self, expect_success: bool) -> String {
    let cmd = duct::cmd("cargo", &self.cargo_args)
      .dir(&self.destination_root_directory)
      .unchecked()
      .stderr_to_stdout()
      .stdout_capture();

    let mut cmd_joined = vec!["cargo".to_string()];
    cmd_joined.extend(self.cargo_args.iter().map(|oss| oss.clone().into_string().expect("failed to convert cmd to string")));
    let cmd_joined = cmd_joined.join(" ");
    println!("$ {}", cmd_joined);

    let cmd_output = cmd.run()
      .expect("failed to run cargo");
    let output = String::from_utf8(cmd_output.stdout)
      .expect("failed to convert stdout to utf8");
    print!("{}", output);

    let success = cmd_output.status.success();
    if expect_success && !success {
      panic!("cargo failed while it should have succeed");
    }
    if !expect_success && success {
      panic!("cargo succeeded while it should have failed");
    }

    format!("$ {}\n{}", cmd_joined, output)
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
      output.apply(self);
    }
  }
}
