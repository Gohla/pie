use std::fmt::Debug;
use std::hash::Hash;
use std::io;
use std::io::{Read, Write};
use std::path::PathBuf;

use pie::{Context, Key, OutputChecker, ResourceChecker, Task};
use pie::resource::file::{FsError, ModifiedChecker};
use pie::task::{AlwaysConsistent, EqualsChecker};

/// Apply a function over over Result/Option/value (kinda like a functor).
trait Apply<A, B> {
  type Output;
  fn apply<F: FnOnce(A) -> B>(self, f: F) -> Self::Output;
}
impl<A, B, E> Apply<A, B> for Result<A, E> {
  type Output = Result<B, E>;
  fn apply<F: FnOnce(A) -> B>(self, f: F) -> Self::Output {
    self.map(|v| f(v))
  }
}
impl<A, B> Apply<A, B> for Option<A> {
  type Output = Option<B>;
  fn apply<F: FnOnce(A) -> B>(self, f: F) -> Self::Output {
    self.map(|v| f(v))
  }
}
impl<A, B> Apply<A, B> for A {
  type Output = B;
  fn apply<F: FnOnce(A) -> B>(self, f: F) -> Self::Output {
    f(self)
  }
}
/// [`Apply`] that returns `Self`.
trait MonoApply<A>: Apply<A, A, Output=Self> {}
impl<A, R: Apply<A, A, Output=Self>> MonoApply<A> for R {}


/// Task that always returns its constant value.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Constant<T>(pub T);
impl<T> Constant<T> {
  pub fn new(val: impl Into<T>) -> Self {
    Self(val.into())
  }
}
impl<T, E> Constant<Result<T, E>> {
  pub fn new_ok(val: impl Into<T>) -> Self {
    Self(Ok(val.into()))
  }
}
impl<T: Key> Task for Constant<T> {
  type Output = T;
  #[inline]
  fn execute<C: Context>(&self, _context: &mut C) -> Self::Output {
    self.0.clone()
  }
}

/// Task that requires another task.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct Require<T, H>(pub T, pub H);
impl<T, H> Require<T, H> {
  #[inline]
  pub fn with_checker(task: T, checker: H) -> Self {
    Self(task, checker)
  }
}
impl<T> Require<T, EqualsChecker> {
  #[inline]
  pub fn new(task: T) -> Self {
    Self(task, EqualsChecker)
  }
}
impl<T: Task, H: OutputChecker<T::Output>> Task for Require<T, H> {
  type Output = T::Output;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    context.require(&self.0, self.1.clone())
  }
}


/// Task that requires another task that returns a `String`, and returns that string in lowercase.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ToLower<T>(pub T);
impl<T: Task> ToLower<T> {
  pub fn new(task: T) -> Self {
    Self(task)
  }
  pub fn from(task: &T) -> Self {
    Self(task.clone())
  }
}
impl<T: Task> Task for ToLower<T> where T::Output: MonoApply<String> + Eq {
  type Output = T::Output;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    context.require(&self.0, EqualsChecker).apply(|s| s.to_lowercase())
  }
}

/// Task that requires another task that returns a `String`, and returns that string in uppercase.
#[derive(Default, Copy, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ToUpper<T>(pub T);
impl<T: Task> ToUpper<T> {
  pub fn new(task: T) -> Self {
    Self(task)
  }
  pub fn from(task: &T) -> Self {
    Self(task.clone())
  }
}
impl<T: Task> Task for ToUpper<T> where T::Output: MonoApply<String> + Eq {
  type Output = T::Output;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    context.require(&self.0, EqualsChecker).apply(|s| s.to_uppercase())
  }
}


/// Task that reads the contents of a file into a `String`.
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ReadFile<H, T>(pub PathBuf, pub H, pub Option<T>);
impl ReadFile<ModifiedChecker, ()> {
  #[inline]
  pub fn new(file: impl Into<PathBuf>) -> Self {
    Self(file.into(), ModifiedChecker, None)
  }
}
impl<H, T> ReadFile<H, T> {
  #[inline]
  pub fn with_checker<HH>(self, checker: HH) -> ReadFile<HH, T> {
    ReadFile(self.0, checker, self.2)
  }
  #[inline]
  pub fn with_origin<TT>(self, origin: TT) -> ReadFile<H, TT> {
    ReadFile(self.0, self.1, Some(origin))
  }
}
impl<H: ResourceChecker<PathBuf, Error=FsError>, T: Task> Task for ReadFile<H, T> {
  type Output = Result<String, H::Error>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    if let Some(origin) = &self.2 {
      // HACK: use AlwaysConsistent to ignore result, but the error of the task may influence us!
      context.require(origin, AlwaysConsistent);
    }
    let (mut file, metadata) = context.read(&self.0, self.1.clone())?.try_into_file_and_metadata()?;
    let mut buf = String::with_capacity(metadata.len() as usize);
    file.read_to_string(&mut buf)?;
    Ok(buf)
  }
}

/// Task that reads the contents of a file into a `Vec<u8>`.
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ReadFileToBytes<H>(pub PathBuf, pub H);
impl<H> ReadFileToBytes<H> {
  #[inline]
  pub fn with_checker(file: impl Into<PathBuf>, checker: H) -> Self {
    Self(file.into(), checker)
  }
}
impl ReadFileToBytes<ModifiedChecker> {
  #[inline]
  pub fn new(file: impl Into<PathBuf>) -> Self {
    Self::with_checker(file, ModifiedChecker)
  }
}
impl<H: ResourceChecker<PathBuf, Error=FsError>> Task for ReadFileToBytes<H> {
  type Output = Result<Vec<u8>, H::Error>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let (mut file, metadata) = context.read(&self.0, self.1.clone())?.try_into_file_and_metadata()?;
    let mut buf = Vec::with_capacity(metadata.len() as usize);
    file.read_to_end(&mut buf)?;
    Ok(buf)
  }
}

/// Task that lists the contents of a directory into a `String`.
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct ListDirectory<H>(pub PathBuf, pub H);
impl<H> ListDirectory<H> {
  #[inline]
  pub fn with_checker(file: impl Into<PathBuf>, checker: H) -> Self {
    Self(file.into(), checker)
  }
}
impl ListDirectory<ModifiedChecker> {
  #[inline]
  pub fn new(file: impl Into<PathBuf>) -> Self {
    Self::with_checker(file, ModifiedChecker)
  }
}
impl<H: ResourceChecker<PathBuf, Error=FsError>> Task for ListDirectory<H> {
  type Output = Result<String, H::Error>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    context.read(&self.0, self.1.clone())?;
    let paths = std::fs::read_dir(&self.0)?;
    let paths: Result<String, io::Error> = paths
      .into_iter()
      .map(|r| r.map(|de| de.path()))
      .try_fold(String::new(), |a, b| Ok(a + &b?.to_string_lossy() + "\n"));
    Ok(paths?)
  }
}


/// Task that requires another task to get a `String` which it writes into a file.
#[derive(Default, Clone, Eq, PartialEq, Hash, Debug)]
pub struct WriteFile<T, H>(pub T, pub PathBuf, pub H);
impl<T, H> WriteFile<T, H> {
  #[inline]
  pub fn with_checker(string_provider: T, file: impl Into<PathBuf>, checker: H) -> Self {
    Self(string_provider, file.into(), checker)
  }
}
impl<T: Task> WriteFile<T, ModifiedChecker> {
  #[inline]
  pub fn new(string_provider: T, file: impl Into<PathBuf>) -> Self {
    Self(string_provider, file.into(), ModifiedChecker)
  }
  #[inline]
  pub fn from(string_provider: &T, file: impl Into<PathBuf>) -> Self {
    Self(string_provider.clone(), file.into(), ModifiedChecker)
  }
}
impl<T: Task<Output=Result<String, FsError>>, H: ResourceChecker<PathBuf, Error=FsError>> Task for WriteFile<T, H> {
  type Output = Result<PathBuf, H::Error>;
  #[inline]
  fn execute<C: Context>(&self, context: &mut C) -> Self::Output {
    let string = context.require(&self.0, EqualsChecker)?;
    context.write(&self.1, self.2.clone(), |file| {
      file.write_all(string.as_bytes())?;
      Ok(())
    })?;
    Ok(self.1.clone())
  }
}


#[cfg(test)]
mod tests {
  use std::convert::Infallible;

  use assert_matches::assert_matches;

  use pie::Pie;

  use super::*;

  #[test]
  fn test_lower() {
    let mut pie = Pie::default();

    let output = pie.new_session().require(&ToLower(Constant("TEST".to_string())));
    assert_eq!(&output, "test");

    let output: Result<String, Infallible> = pie.new_session().require(&ToLower(Constant(Ok("TEST".to_string()))));
    assert_eq!(output, Ok("test".to_string()));

    let output: Result<String, FsError> = pie.new_session().require(&ToLower(ReadFile::new("nope.txt")));
    assert_matches!(output, Err(_));
  }
}
