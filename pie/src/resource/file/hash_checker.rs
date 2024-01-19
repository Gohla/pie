use std::fmt::Debug;
use std::io::Seek;

use sha2::{Digest, Sha256};

use super::*;

/// Filesystem [resource checker](ResourceChecker) that hashes file contents and directory listings and compares hashes.
#[derive(Default, Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct HashChecker;

impl ResourceChecker<PathBuf> for HashChecker {
  type Stamp = Option<[u8; 32]>;
  type Error = FsError;

  #[inline]
  fn stamp<RS: ResourceState<PathBuf>>(&self, path: &PathBuf, state: &mut RS) -> Result<Self::Stamp, Self::Error> {
    self.hash(path, &mut path.read(state)?)
  }
  #[inline]
  fn stamp_reader(&self, path: &PathBuf, open_read: &mut OpenRead) -> Result<Self::Stamp, Self::Error> {
    let hash = self.hash(path, open_read);
    open_read.rewind()?; // Rewind to restore the file (if any) into a fresh state.
    hash
  }
  #[inline]
  fn stamp_writer(&self, path: &PathBuf, mut file: File) -> Result<Self::Stamp, Self::Error> {
    // Note: we cannot assume `file` exists because it could have been removed before passing it to this method.
    if !exists(path)? {
      return Ok(None);
    }

    file.rewind()?; // Rewind to restore the file into a fresh state.
    let hash = self.hash_file(&mut BufReader::new(file))?;
    Ok(Some(hash))
  }

  #[inline]
  #[allow(refining_impl_trait)]
  fn check<RS: ResourceState<PathBuf>>(
    &self,
    path: &PathBuf,
    state: &mut RS,
    stamp: &Self::Stamp,
  ) -> Result<Option<Self::Stamp>, Self::Error> {
    let hash = self.hash(path, &mut path.read(state)?)?;
    let inconsistency = if hash != *stamp {
      Some(hash)
    } else {
      None
    };
    Ok(inconsistency)
  }

  #[inline]
  fn wrap_error(&self, error: FsError) -> Self::Error { error }
}

impl HashChecker {
  fn hash(&self, path: &PathBuf, open_read: &mut OpenRead) -> Result<Option<[u8; 32]>, FsError> {
    let hash = match open_read {
      OpenRead::File(ref mut file, _) => Some(self.hash_file(file)?),
      OpenRead::Directory(_) => Some(self.hash_directory(path)?),
      OpenRead::NonExistent => None
    };
    Ok(hash)
  }
  fn hash_file(&self, file: &mut BufReader<File>) -> Result<[u8; 32], FsError> {
    let mut hasher = Sha256::new();
    io::copy(file, &mut hasher)?;
    Ok(hasher.finalize().into())
  }
  fn hash_directory(&self, path: &PathBuf) -> Result<[u8; 32], FsError> {
    let mut hasher = Sha256::new();
    for entry in fs::read_dir(path)?.into_iter() {
      hasher.update(entry?.file_name().as_encoded_bytes());
    }
    Ok(hasher.finalize().into())
  }
}


#[cfg(test)]
mod test {
  use std::fs::{remove_file, write};

  use assert_matches::assert_matches;
  use testresult::TestResult;

  use dev_util::create_temp_file;

  use crate::trait_object::collection::TypeToAnyMap;

  use super::*;

  #[test]
  fn test_hash_checker() -> TestResult {
    let checker = HashChecker;
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

    write(&path, "Change hash checker")?;
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
      write(&path, "More changes for hash checker")?;

      let new_stamp = checker.stamp_writer(&path, file)?;
      assert_matches!(checker.check(&path, &mut state, &new_stamp)?, None);
      assert_matches!(checker.check(&path, &mut state, &stamp)?, Some(s) if s == new_stamp);
    }

    Ok(())
  }
}
