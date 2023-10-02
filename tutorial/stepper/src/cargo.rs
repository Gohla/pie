use std::ffi::OsString;
use std::fmt::{Display, Formatter};
use std::path::PathBuf;

use anyhow::Context;
use duct::Expression;

#[derive(Clone)]
pub struct RunCargo {
  cmd: Expression,
  cmd_joined: String,
}

impl Display for RunCargo {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    write!(f, "$ {}", self.cmd_joined)
  }
}

impl RunCargo {
  pub fn new(cargo_args: impl IntoIterator<Item=impl Into<OsString>> + Clone, destination_directory: &PathBuf) -> anyhow::Result<RunCargo> {
    let cmd = duct::cmd("cargo", cargo_args.clone())
      .dir(destination_directory)
      .unchecked()
      .stderr_to_stdout()
      .stdout_capture();

    let mut cmd_joined = vec!["cargo".to_string()];
    cmd_joined.extend(cargo_args.into_iter().map(|oss| oss.into().into_string()
      .expect("failed to convert cmd to string")));
    let cmd_joined = cmd_joined.join(" ");
    Ok(Self { cmd, cmd_joined })
  }

  pub fn run(&self, expect_success: Option<bool>) -> anyhow::Result<(String, bool)> {
    let cmd_output = self.cmd.run()
      .context("failed to run cargo")?;
    let output = String::from_utf8(cmd_output.stdout)
      .context("failed to convert stdout to utf8")?;
    let cargo_success = cmd_output.status.success();
    let success = expect_success.map(|e| e == cargo_success).unwrap_or(true);
    Ok((output, success))
  }
}
