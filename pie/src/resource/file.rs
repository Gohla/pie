use std::convert::Infallible;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::{self, File, Metadata, OpenOptions};
use std::io::{self, BufReader, Seek};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::{Resource, ResourceChecker, ResourceState};

#[cfg(feature = "file_hash_checker")]
pub mod hash_checker;

/// Filesystem resource implementation. Files and directories can be opened for reading. Only files can be opened for
/// writing.
///
/// Files are opened for writing with `File::create`, meaning that it will create the file if it does not exist, and
/// truncate it if it does exist. They are also opened with read access, so that checkers can read file contents.
impl Resource for PathBuf {
  type Reader<'rc> = OpenRead;
  type Writer<'r> = File;
  type Error = FsError;

  /// Opens this path for reading, returning an [open reader](OpenRead).
  ///
  /// # Errors
  ///
  /// Returns an error if getting metadata for this path failed, or if opening this path as a file failed.
  #[inline]
  fn read<RS: ResourceState<Self>>(&self, _state: &mut RS) -> Result<OpenRead, FsError> {
    OpenRead::new(self)
  }

  /// Opens this path for writing, returning a [`File`].
  ///
  /// # Errors
  ///
  /// Returns an error if opening this path as a file failed, or if a directory already exists at this path.
  #[inline]
  fn write<RS: ResourceState<Self>>(&self, _state: &mut RS) -> Result<File, FsError> {
    if let Some(metadata) = metadata(self)? {
      if metadata.is_dir() {
        return Err(FsError(io::ErrorKind::AlreadyExists));
      }
    }
    // Note: open with `read` option so that checkers can read file contents.
    let file = OpenOptions::new().write(true).create(true).truncate(true).read(true).open(self)?;
    Ok(file)
  }
}

/// A potentially opened filesystem path for reading, representing:
///
/// - a [file](OpenRead::File) as a [buffered reader](BufReader<File>) and [Metadata],
/// - a [directory](OpenRead::Directory) as [Metadata],
/// - [nothing](OpenRead::NonExistent) indicating no file nor directory exists.
pub enum OpenRead {
  File(BufReader<File>, Metadata),
  Directory(Metadata),
  NonExistent,
}
impl OpenRead {
  /// Attempt to open file or directory at given `path` for reading.
  ///
  /// # Errors
  ///
  /// Returns an error if getting the metadata for `path` failed, or if opening the file failed.
  #[inline]
  fn new(path: impl AsRef<Path>) -> Result<Self, FsError> {
    let Some(metadata) = metadata(&path)? else {
      return Ok(Self::NonExistent);
    };
    let open_read = if metadata.is_file() {
      let file = File::open(path)?;
      OpenRead::File(BufReader::new(file), metadata)
    } else {
      OpenRead::Directory(metadata)
    };
    Ok(open_read)
  }

  /// Returns `true` if this is a file, `false` otherwise.
  pub fn is_file(&self) -> bool {
    matches!(self, Self::File(_, _))
  }
  /// Returns `true` if this is a directory, `false` otherwise.
  pub fn is_directory(&self) -> bool {
    matches!(self, Self::Directory(_))
  }
  /// Returns `true` if this is a file or directory, `false` otherwise.
  pub fn exists(&self) -> bool {
    !matches!(self, Self::NonExistent)
  }

  /// Returns `Some(&mut file)` if this is a file, `None` otherwise.
  #[inline]
  pub fn as_file(&mut self) -> Option<&mut BufReader<File>> {
    match self {
      Self::File(ref mut file, _) => Some(file),
      _ => None,
    }
  }
  /// Returns `Some((&mut file, &metadata))` if this is a file, `None` otherwise.
  #[inline]
  pub fn as_file_and_metadata(&mut self) -> Option<(&mut BufReader<File>, &Metadata)> {
    match self {
      Self::File(ref mut file, ref metadata) => Some((file, metadata)),
      _ => None,
    }
  }
  /// Returns `Some(&metadata)` if this is a directory, `None` otherwise.
  #[inline]
  pub fn as_directory(&self) -> Option<&Metadata> {
    match self {
      Self::Directory(metadata) => Some(metadata),
      _ => None,
    }
  }
  /// Returns `Some(&metadata)` if this is a file or directory, `None` otherwise.
  #[inline]
  pub fn as_metadata(&self) -> Option<&Metadata> {
    match self {
      Self::File(_, metadata) => Some(metadata),
      Self::Directory(metadata) => Some(metadata),
      _ => None,
    }
  }

  /// Returns `Some(file)` if this is a file, `None` otherwise.
  #[inline]
  pub fn into_file(self) -> Option<BufReader<File>> {
    match self {
      Self::File(file, _) => Some(file),
      _ => None,
    }
  }
  /// Returns `Some((file, metadata))` if this is a file, `None` otherwise.
  #[inline]
  pub fn into_file_and_metadata(self) -> Option<(BufReader<File>, Metadata)> {
    match self {
      Self::File(file, metadata) => Some((file, metadata)),
      _ => None,
    }
  }
  /// Returns `Some(metadata)` if this is a directory, `None` otherwise.
  #[inline]
  pub fn into_directory(self) -> Option<Metadata> {
    match self {
      Self::Directory(metadata) => Some(metadata),
      _ => None,
    }
  }
  /// Returns `Some(metadata)` if this is a file or directory, `None` otherwise.
  #[inline]
  pub fn into_metadata(self) -> Option<Metadata> {
    match self {
      Self::File(_, metadata) => Some(metadata),
      Self::Directory(metadata) => Some(metadata),
      _ => None,
    }
  }

  /// Returns `Ok(file)` if this is a file, `Err(FsError(io::ErrorKind::NotFound))` otherwise.
  #[inline]
  pub fn try_into_file(self) -> Result<BufReader<File>, FsError> {
    self.into_file().ok_or(FsError(io::ErrorKind::NotFound))
  }
  /// Returns `Ok((file, metadata))` if this is a file, `Err(FsError(io::ErrorKind::NotFound))` otherwise.
  #[inline]
  pub fn try_into_file_and_metadata(self) -> Result<(BufReader<File>, Metadata), FsError> {
    self.into_file_and_metadata().ok_or(FsError(io::ErrorKind::NotFound))
  }
  /// Returns `Ok(metadata)` if this is a directory, `Err(FsError(io::ErrorKind::NotFound))` otherwise.
  #[inline]
  pub fn try_into_directory(self) -> Result<Metadata, FsError> {
    self.into_directory().ok_or(FsError(io::ErrorKind::NotFound))
  }
  /// Returns `Ok(metadata)` if this is a file or directory, `Err(FsError(io::ErrorKind::NotFound))` otherwise.
  #[inline]
  pub fn try_into_metadata(self) -> Result<Metadata, FsError> {
    self.into_metadata().ok_or(FsError(io::ErrorKind::NotFound))
  }

  /// Rewinds the buffered file reader if this is a file. Does nothing if not a file.
  pub fn rewind(&mut self) -> Result<(), FsError> {
    match self {
      Self::File(file, _) => file.rewind()?,
      _ => {}
    }
    Ok(())
  }
}

/// Filesystem resource error, a newtype wrapper around `io::ErrorKind`.
///
/// # Implementation Notes
///
/// We need a type that implements `Error` because we want to use the type as an error, and we need the type to
/// implement `Clone` because task outputs need to implement `Clone`, and this error can be used as a task output.
///
/// We cannot use `io::ErrorKind` because we cannot implement `Error` for it due to Rust's orphan rule. We cannot use
/// `io::Error` because it is not `Clone`. Therefore, we create this newtype wrapper around `io::ErrorKind` to implement
/// `Error`.
///
/// This error ignores the custom error payload from `io::Error` as it is not `Clone`, and the custom message from
/// `io::Error` as it is not accessible.
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[repr(transparent)]
pub struct FsError(io::ErrorKind);

impl Error for FsError {}

impl From<io::ErrorKind> for FsError {
  #[inline]
  fn from(value: io::ErrorKind) -> Self { Self(value) }
}
impl From<io::Error> for FsError {
  #[inline]
  fn from(value: io::Error) -> Self { Self(value.kind()) }
}
impl From<Infallible> for FsError {
  #[inline]
  fn from(value: Infallible) -> Self { value.into() }
}
impl From<FsError> for io::ErrorKind {
  #[inline]
  fn from(value: FsError) -> Self { value.0 }
}
impl From<FsError> for io::Error {
  #[inline]
  fn from(value: FsError) -> Self { value.0.into() }
}
impl Display for FsError {
  #[inline]
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result { self.0.fmt(f) }
}


/// Filesystem [resource checker](ResourceChecker) that compares file or directory last modified dates.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ModifiedChecker;

impl ResourceChecker<PathBuf> for ModifiedChecker {
  type Stamp = Option<SystemTime>;
  type Error = FsError;

  #[inline]
  fn stamp<RS: ResourceState<PathBuf>>(&self, path: &PathBuf, _state: &mut RS) -> Result<Self::Stamp, Self::Error> {
    let modified = metadata(path)?.map(|m| m.modified()).transpose()?;
    Ok(modified)
  }
  #[inline]
  fn stamp_reader(&self, _path: &PathBuf, open_read: &mut OpenRead) -> Result<Self::Stamp, Self::Error> {
    let modified = open_read.as_metadata().map(|m| m.modified()).transpose()?;
    Ok(modified)
  }
  #[inline]
  fn stamp_writer(&self, path: &PathBuf, file: File) -> Result<Self::Stamp, Self::Error> {
    // Note: we first need to confirm `file` still exists. If `file` does not exist, `file.metadata()` returns stale
    //       metadata instead of returning an error, resulting in an inconsistent stamp.
    if !exists(path)? {
      return Ok(None);
    }
    Ok(Some(file.metadata()?.modified()?))
  }

  type Inconsistency<'i> = Self::Stamp;
  #[inline]
  fn check<RS: ResourceState<PathBuf>>(
    &self,
    path: &PathBuf,
    _state: &mut RS,
    stamp: &Self::Stamp,
  ) -> Result<Option<Self::Stamp>, Self::Error> {
    let modified = metadata(path)?.map(|m| m.modified()).transpose()?;
    let inconsistency = if modified != *stamp {
      Some(modified)
    } else {
      None
    };
    Ok(inconsistency)
  }

  #[inline]
  fn wrap_error(&self, error: FsError) -> Self::Error { error }
}

/// Filesystem [resource checker](ResourceChecker) that compares whether a file or directory exists.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ExistsChecker;

impl ResourceChecker<PathBuf> for ExistsChecker {
  type Stamp = bool;
  type Error = FsError;

  #[inline]
  fn stamp<RS: ResourceState<PathBuf>>(&self, path: &PathBuf, _state: &mut RS) -> Result<Self::Stamp, Self::Error> {
    let exists = exists(path)?;
    Ok(exists)
  }
  #[inline]
  fn stamp_reader(&self, _path: &PathBuf, open_read: &mut OpenRead) -> Result<Self::Stamp, Self::Error> {
    let exists = open_read.exists();
    Ok(exists)
  }
  #[inline]
  fn stamp_writer(&self, path: &PathBuf, _file: File) -> Result<Self::Stamp, Self::Error> {
    // Note: we cannot assume `_file` exists because it could have been removed before passing it to this method.
    let exists = exists(path)?;
    Ok(exists)
  }

  type Inconsistency<'i> = Self::Stamp;
  #[inline]
  fn check<RS: ResourceState<PathBuf>>(
    &self,
    path: &PathBuf,
    _state: &mut RS,
    stamp: &Self::Stamp,
  ) -> Result<Option<Self::Stamp>, Self::Error> {
    let exists = metadata(path)?.is_some();
    let inconsistency = if exists != *stamp {
      Some(exists)
    } else {
      None
    };
    Ok(inconsistency)
  }

  #[inline]
  fn wrap_error(&self, error: FsError) -> Self::Error { error }
}


/// Gets the metadata for given `path`, returning:
///
/// - `Ok(Some(metadata))` if a file or directory exists at given path,
/// - `Ok(None)` if no file or directory exists at given path,
/// - `Err(e)` if there was an error getting the metadata for given path.
#[inline]
fn metadata(path: impl AsRef<Path>) -> Result<Option<Metadata>, io::Error> {
  match fs::metadata(path) {
    Ok(m) => Ok(Some(m)),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
    Err(e) => Err(e),
  }
}

/// Checks whether a file or directory exists at `path`, returning `true` if it exists, `false` otherwise.
///
/// # Errors
///
/// Returns an error if getting the metadata for `path` failed.
#[inline]
fn exists(path: impl AsRef<Path>) -> Result<bool, io::Error> {
  match fs::metadata(path) {
    Ok(_) => Ok(true),
    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(false),
    Err(e) => Err(e),
  }
}


#[cfg(test)]
mod test {
  use std::fs::{create_dir_all, remove_dir, remove_file, write};
  use std::io::{Read, Write};

  use assert_matches::assert_matches;

  use dev_util::{create_temp_dir, create_temp_file, write_until_modified};

  use crate::trait_object::collection::TypeToAnyMap;

  use super::*;

  #[test]
  fn test_resource_read() -> Result<(), io::Error> {
    let temp_path = create_temp_file()?.into_temp_path();
    let path = temp_path.to_path_buf();
    let mut state = TypeToAnyMap::default();

    write(&path, "Hello, World!")?;
    {
      let mut open_read = path.read(&mut state)?;
      assert_matches!(open_read.as_metadata(), Some(metadata) => {
        assert!(metadata.is_file());
      });
      assert_matches!(open_read.as_file(), Some(file) => {
        let mut string = String::new();
        file.read_to_string(&mut string)?;
        assert_eq!(&string, "Hello, World!");
      });
    }

    remove_file(&path)?;
    {
      let mut open_read = path.read(&mut state)?;
      assert_matches!(open_read.as_metadata(), None);
      assert_matches!(open_read.as_file(), None);
    }

    create_dir_all(&path)?;
    {
      let mut open_read = path.read(&mut state)?;
      assert_matches!(open_read.as_metadata(), Some(metadata) => {
        assert!(metadata.is_dir());
      });
      assert_matches!(open_read.as_file(), None);
    }

    Ok(())
  }

  #[test]
  fn test_resource_write() -> Result<(), io::Error> {
    let temp_path = create_temp_file()?.into_temp_path();
    let path = temp_path.to_path_buf();
    let mut state = TypeToAnyMap::default();

    {
      let mut open_write = path.write(&mut state)?;
      open_write.write_all("Hello, World!".as_bytes())?;
      drop(open_write);

      let mut open_read = path.read(&mut state)?;
      let mut string = String::new();
      open_read.as_file().unwrap().read_to_string(&mut string)?;
      assert_eq!(&string, "Hello, World!");
    }

    remove_file(&path)?;
    {
      let _open_write = path.write(&mut state)?;
      assert!(path.exists());
    }

    remove_file(&path)?;
    create_dir_all(&path)?;
    assert_matches!(path.write(&mut state), Err(FsError(io::ErrorKind::AlreadyExists)));

    Ok(())
  }


  #[test]
  fn test_modified_checker() -> Result<(), io::Error> {
    let checker = ModifiedChecker;
    let temp_path = create_temp_file()?.into_temp_path();
    let path = temp_path.to_path_buf();
    let mut state = TypeToAnyMap::default();

    let stamp = {
      let stamp = checker.stamp(&path, &mut state)?;
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      let stamp = checker.stamp_reader(&path, &mut path.read(&mut state)?)?;
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      let stamp = checker.stamp_writer(&path, File::open(&path)?)?;
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      stamp
    };

    write_until_modified(&path, format!("{:?}", stamp))?;
    let stamp = {
      let new_stamp = checker.stamp(&path, &mut state)?;
      // Consistent with `new_stamp` that was made after modifying the file.
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      // But not with (old) `stamp` that was made before modifying the file.
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      let new_stamp = checker.stamp_reader(&path, &mut path.read(&mut state)?)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      let new_stamp = checker.stamp_writer(&path, File::open(&path)?)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      new_stamp
    };

    remove_file(&path)?;
    let stamp = {
      let new_stamp = checker.stamp(&path, &mut state)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      let new_stamp = checker.stamp_reader(&path, &mut path.read(&mut state)?)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      // Note: can't test `stamp_writer` because the file does not exist.

      new_stamp
    };
    assert_matches!(stamp, None); // Stamp is `None` because file does not exist.

    let stamp = { // Test `stamp_writer` when removing file after creating a writer.
      let file = path.write(&mut state)?;
      assert!(path.exists());
      remove_file(&path)?;
      assert!(!path.exists());

      let new_stamp = checker.stamp_writer(&path, file)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      // This matches the (old) `stamp` because the file is removed in both cases.
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      new_stamp
    };

    { // Test `stamp_writer` when modifying file after creating a writer.
      let file = path.write(&mut state)?;
      write_until_modified(&path, format!("{:?}", stamp))?;

      let new_stamp = checker.stamp_writer(&path, file)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);
    }

    Ok(())
  }

  #[test]
  fn test_exists_file_stamper() -> Result<(), io::Error> {
    let checker = ExistsChecker;
    let temp_path = create_temp_file()?.into_temp_path();
    let path = temp_path.to_path_buf();
    let mut state = TypeToAnyMap::default();

    let stamp = {
      let stamp = checker.stamp(&path, &mut state)?;
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      let stamp = checker.stamp_reader(&path, &mut path.read(&mut state)?)?;
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      let stamp = checker.stamp_writer(&path, File::open(&path)?)?;
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      stamp
    };

    remove_file(&path)?;
    let stamp = {
      let new_stamp = checker.stamp(&path, &mut state)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      let new_stamp = checker.stamp_reader(&path, &mut path.read(&mut state)?)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);

      // Note: can't test `stamp_writer` because the file does not exist.

      new_stamp
    };
    assert_matches!(stamp, false); // Stamp is `false` because file does not exist.

    let stamp = { // Test `stamp_writer` when removing file after creating a writer.
      let file = path.write(&mut state)?;
      assert!(path.exists());
      remove_file(&path)?;
      assert!(!path.exists());

      let new_stamp = checker.stamp_writer(&path, file)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      // This matches the (old) `stamp` because the file is removed in both cases.
      assert_matches!(checker.check(&path, &mut state, &stamp)?, None);

      new_stamp
    };
    assert_matches!(stamp, false);

    Ok(())
  }


  #[test]
  fn test_metadata() -> Result<(), io::Error> {
    let file = create_temp_file()?;
    assert_matches!(metadata(&file)?, Some(metadata) => {
      assert!(metadata.is_file());
    });
    remove_file(&file)?;
    assert_matches!(metadata(&file)?, None);

    let dir = create_temp_dir()?;
    assert_matches!(metadata(&dir)?, Some(metadata) => {
      assert!(metadata.is_dir());
    });
    remove_dir(&dir)?;
    assert_matches!(metadata(&dir)?, None);

    Ok(())
  }

  #[test]
  fn test_exists() -> Result<(), io::Error> {
    let file = create_temp_file()?;
    assert!(exists(&file)?);
    remove_file(&file)?;
    assert!(!exists(&file)?);

    let dir = create_temp_dir()?;
    assert!(exists(&dir)?);
    remove_dir(&dir)?;
    assert!(!exists(&dir)?);

    Ok(())
  }
}
