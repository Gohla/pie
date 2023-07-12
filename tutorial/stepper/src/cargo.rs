use std::fmt::{Display, Formatter};

use anyhow::Context;
use duct::Expression;

use crate::stepper::Stepper;

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
  pub fn new(stepper: &Stepper) -> anyhow::Result<RunCargo> {
    let cmd = duct::cmd("cargo", &stepper.cargo_args)
      .dir(&stepper.destination_directory)
      .unchecked()
      .stderr_to_stdout()
      .stdout_capture();

    let mut cmd_joined = vec!["cargo".to_string()];
    cmd_joined.extend(stepper.cargo_args.iter().map(|oss| oss.clone().into_string()
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
