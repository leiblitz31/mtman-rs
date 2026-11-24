//! File operations.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// A file entry.
#[derive(Debug, Clone, serde::Serialize)]
pub struct FileEntry {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub is_file: bool,
    pub is_symlink: bool,
    pub size: u64,
    pub mtime: u64,
    pub mode: u32,
}

/// Read a directory and return its entries.
pub fn list_dir(path: &Path) -> io::Result<Vec<FileEntry>> {
    let mut entries = Vec::new();
    for entry in fs::read_dir(path)? {
        let entry = entry?;
        let meta = entry.metadata()?;
        let ft = meta.file_type();
        let path = entry.path();
        entries.push(FileEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            path,
            is_dir: ft.is_dir(),
            is_file: ft.is_file(),
            is_symlink: ft.is_symlink(),
            size: meta.len(),
            mtime: meta
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0),
            mode: 0, // TODO: fill from stat
        });
    }
    Ok(entries)
}

/// Read a file into a Vec<u8>.
pub fn read_file(path: &Path) -> io::Result<Vec<u8>> {
    fs::read(path)
}

/// Write data to a file.
pub fn write_file(path: &Path, data: &[u8]) -> io::Result<()> {
    fs::write(path, data)
}

/// Copy a file.
pub fn copy_file(src: &Path, dst: &Path) -> io::Result<()> {
    fs::copy(src, dst)?;
    Ok(())
}

/// Remove a file.
pub fn remove_file(path: &Path) -> io::Result<()> {
    fs::remove_file(path)
}

/// Remove a directory.
pub fn remove_dir(path: &Path) -> io::Result<()> {
    fs::remove_dir_all(path)
}

/// Rename a file.
pub fn rename(src: &Path, dst: &Path) -> io::Result<()> {
    fs::rename(src, dst)
}

/// Create a directory.
pub fn create_dir(path: &Path) -> io::Result<()> {
    fs::create_dir_all(path)
}
