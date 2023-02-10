use std::fmt::{Debug, Formatter};
use std::fs::File;
use std::path::PathBuf;
use std::time::SystemTime;

use serde::Serializer;

// File stampers

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FileStamper {
  Exists,
  Modified,
  #[cfg(feature = "recursive_stampers")]
  ModifiedRecursive,
  #[cfg(feature = "hash_stampers")]
  Hash,
  #[cfg(all(feature = "hash_stampers", feature = "recursive_stampers"))]
  HashRecursive,
}

impl FileStamper {
  pub fn stamp(&self, path: &PathBuf) -> Result<FileStamp, std::io::Error> {
    match self {
      FileStamper::Exists => {
        Ok(FileStamp::Exists(path.try_exists()?))
      }
      FileStamper::Modified => {
        Ok(FileStamp::Modified(File::open(path)?.metadata()?.modified()?))
      }
      #[cfg(feature = "recursive_stampers")]
      FileStamper::ModifiedRecursive => {
        use walkdir::WalkDir;
        let mut latest_modification_date = SystemTime::UNIX_EPOCH;
        for entry in WalkDir::new(path).into_iter() {
          let entry_modification_date = entry?.metadata()?.modified()?;
          if entry_modification_date > latest_modification_date {
            latest_modification_date = entry_modification_date;
          }
        }
        Ok(FileStamp::Modified(latest_modification_date))
      }
      #[cfg(feature = "hash_stampers")]
      FileStamper::Hash => {
        use sha2::{Digest, Sha256};
        let mut file = File::open(path)?;
        let mut hasher = Sha256::new();
        if file.metadata()?.is_file() {
          std::io::copy(&mut file, &mut hasher)?;
        } else {
          drop(file); // Drop file because we have to re-open it with `std::fs::read_dir`.
          Self::hash_directory(&mut hasher, path)?;
        }
        Ok(FileStamp::Hash(hasher.finalize().into()))
      }
      #[cfg(all(feature = "hash_stampers", feature = "recursive_stampers"))]
      FileStamper::HashRecursive => {
        use sha2::{Digest, Sha256};
        use walkdir::WalkDir;
        let mut hasher = Sha256::new();
        for entry in WalkDir::new(path).into_iter() {
          let mut file = File::open(entry?.path())?;
          // Skip hashing directories: added/removed files are already represented by hash changes.
          if !file.metadata()?.is_file() { continue; }
          std::io::copy(&mut file, &mut hasher)?;
        }
        Ok(FileStamp::Hash(hasher.finalize().into()))
      }
    }
  }

  #[cfg(feature = "hash_stampers")]
  fn hash_directory(hasher: &mut sha2::Sha256, path: &PathBuf) -> Result<(), std::io::Error> {
    use sha2::Digest;
    for entry in std::fs::read_dir(path)?.into_iter() {
      hasher.update(entry?.file_name().to_string_lossy().as_bytes());
    }
    Ok(())
  }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FileStamp {
  Exists(bool),
  Modified(SystemTime),
  Hash([u8; 32]),
}

impl Debug for FileStamp {
  fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
    match self {
      FileStamp::Exists(b) => {
        f.serialize_str("Exists(")?;
        b.fmt(f)?;
      }
      FileStamp::Modified(st) => {
        f.serialize_str("Modified(")?;
        st.fmt(f)?;
      }
      FileStamp::Hash(h) => {
        f.serialize_str("Hash(")?;
        for b in h.chunks(2) {
          match b {
            [b1, b2] => write!(f, "{:02x}", *b1 as u16 + *b2 as u16)?,
            [b] => write!(f, "{:02x}", b)?,
            _ => {}
          }
        }
      }
    }
    f.serialize_str(")")
  }
}


// Output stampers

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OutputStamper {
  Inconsequential,
  Equals,
}

impl OutputStamper {
  pub fn stamp<O>(&self, output: O) -> OutputStamp<O> {
    match self {
      OutputStamper::Inconsequential => OutputStamp::Inconsequential,
      OutputStamper::Equals => OutputStamp::Equals(output),
    }
  }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OutputStamp<O> {
  Inconsequential,
  Equals(O),
}
