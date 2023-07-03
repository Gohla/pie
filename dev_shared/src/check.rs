use std::fmt::Debug;

use crate::task::CommonOutput;

pub trait CheckErrorExt<T> {
  fn check(self) -> T;
}

impl<T: Debug> CheckErrorExt<T> for Result<T, std::io::Error> {
  fn check(self) -> T {
    self.expect("failed to perform io operation")
  }
}

impl<T: Debug> CheckErrorExt<T> for Result<T, std::io::ErrorKind> {
  fn check(self) -> T {
    self.expect("failed to perform io operation")
  }
}

impl<T: Debug> CheckErrorExt<T> for Result<T, ()> {
  fn check(self) -> T {
    self.expect("something failed")
  }
}

impl CheckErrorExt<()> for CommonOutput {
  fn check(self) -> () {
    match self {
      CommonOutput::ReadStringFromFile(r) => { r.check(); }
      CommonOutput::WriteStringToFile(r) => { r.check(); }
      CommonOutput::ListDirectory(r) => { r.check(); }
      CommonOutput::ToLowerCase(r) => { r.check(); }
      CommonOutput::ToUpperCase(r) => { r.check(); }
      _ => {}
    };
    ()
  }
}
