use std::fmt::Debug;

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
