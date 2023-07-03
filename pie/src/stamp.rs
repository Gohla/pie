use std::fmt::{Debug, Formatter};
use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::time::SystemTime;

use serde::Serializer;

use crate::fs::{metadata, open_if_file};

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
  pub fn stamp(&self, path: impl AsRef<Path>) -> Result<FileStamp, io::Error> {
    match self {
      FileStamper::Exists => {
        Ok(FileStamp::Exists(path.as_ref().try_exists()?))
      }
      FileStamper::Modified => {
        let Some(metadata) = metadata(path)? else {
          return Ok(FileStamp::Modified(None));
        };
        Ok(FileStamp::Modified(Some(metadata.modified()?)))
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
        Ok(FileStamp::Modified(Some(latest_modification_date)))
      }
      #[cfg(feature = "hash_stampers")]
      FileStamper::Hash => {
        let Some(metadata) = metadata(&path)? else {
          return Ok(FileStamp::Hash(None));
        };

        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        if metadata.is_file() {
          let mut file = File::open(&path)?;
          io::copy(&mut file, &mut hasher)?;
        } else {
          for entry in fs::read_dir(path)?.into_iter() {
            hasher.update(entry?.file_name().to_string_lossy().as_bytes());
          }
        }
        Ok(FileStamp::Hash(Some(hasher.finalize().into())))
      }
      #[cfg(all(feature = "hash_stampers", feature = "recursive_stampers"))]
      FileStamper::HashRecursive => {
        use sha2::{Digest, Sha256};
        use walkdir::WalkDir;
        let mut hasher = Sha256::new();
        for entry in WalkDir::new(&path).into_iter() {
          if let Some(mut file) = open_if_file(entry?.path())? {
            io::copy(&mut file, &mut hasher)?;
          }
        }
        Ok(FileStamp::Hash(Some(hasher.finalize().into())))
      }
    }
  }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum FileStamp {
  Exists(bool),
  Modified(Option<SystemTime>),
  Hash(Option<[u8; 32]>),
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
        match h {
          Some(h) => {
            f.serialize_str("Some(")?;
            for b in h.chunks(2) {
              match b {
                [b1, b2] => write!(f, "{:02x}", *b1 as u16 + *b2 as u16)?,
                [b] => write!(f, "{:02x}", b)?,
                _ => {}
              }
            }
            f.serialize_str(")")?;
          }
          h => h.fmt(f)?,
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

#[derive(Clone, Eq, PartialEq, Debug)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum OutputStamp<O> {
  Inconsequential,
  Equals(O),
}

impl<O> OutputStamp<O> {
  pub fn as_ref(&self) -> OutputStamp<&O> {
    match self {
      OutputStamp::Inconsequential => OutputStamp::Inconsequential,
      OutputStamp::Equals(o) => OutputStamp::Equals(o),
    }
  }
}


#[cfg(test)]
mod test {
  use std::fs;

  use dev_shared::fs::{create_temp_file, write_until_modified};

  use super::*;

  #[test]
  fn test_exists_file_stamper() {
    let stamper = FileStamper::Exists;
    let temp_file = create_temp_file();
    let stamp = stamper.stamp(&temp_file).expect("failed to stamp");
    assert_eq!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"));

    fs::remove_file(&temp_file).expect("failed to delete temporary file");
    assert_ne!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"));
  }

  #[test]
  fn test_modified_file_stamper() {
    let stamper = FileStamper::Modified;
    let temp_file = create_temp_file();
    let stamp = stamper.stamp(&temp_file).expect("failed to stamp");
    assert_eq!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"));

    write_until_modified(&temp_file, format!("{:?}", stamp)).expect("failed to write to temporary file");
    assert_ne!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"), "modified stamp is equal after modifying file");

    fs::remove_file(&temp_file).expect("failed to delete temporary file");
    assert_ne!(stamp, stamper.stamp(&temp_file).expect("failed to stamp"), "modified stamp is equal after removing file");
  }

  #[test]
  fn test_inconsequential_output_stamper() {
    let stamper = OutputStamper::Inconsequential;
    let stamp = stamper.stamp(&1);
    assert_eq!(stamp, stamper.stamp(&1));
    assert_eq!(stamp, stamper.stamp(&2));
  }

  #[test]
  fn test_equals_output_stamper() {
    let stamper = OutputStamper::Equals;
    let stamp = stamper.stamp(&1);
    assert_eq!(stamp, stamper.stamp(&1));
    assert_ne!(stamp, stamper.stamp(&2));
  }
}
