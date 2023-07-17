use std::ffi::OsStr;
use std::fs::{File, OpenOptions};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

pub fn open_writable_file(file_path: impl AsRef<Path>, append: bool) -> anyhow::Result<File> {
  let file_path = file_path.as_ref();
  fs::create_dir_all(file_path.parent().unwrap())?;
  let file = OpenOptions::new()
    .write(true)
    .create(true)
    .append(append)
    .truncate(!append)
    .open(file_path)?;
  Ok(file)
}

pub fn write_to_file(buf: &[u8], file_path: impl AsRef<Path>, append: bool) -> anyhow::Result<()> {
  let mut file = open_writable_file(file_path, append)?;
  file.write_all(buf)?;
  Ok(())
}

pub fn add_extension(path: &mut PathBuf, extension: impl AsRef<Path>) {
  match path.extension() {
    Some(ext) => {
      let mut ext = ext.to_os_string();
      ext.push(".");
      ext.push(extension.as_ref());
      path.set_extension(ext)
    }
    None => path.set_extension(extension.as_ref()),
  };
}

pub fn is_hidden(file_name: &OsStr) -> bool {
  file_name.to_str()
    .map(|f| f.starts_with(".") || f == "target")
    .unwrap_or(false)
}
