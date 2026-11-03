//! `.qj` 写入用的同目录临时文件，保证并发写不共用路径且失败不留垃圾。

use std::fs::{self, File, OpenOptions};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMPORARY: AtomicU64 = AtomicU64::new(0);

pub(super) struct TemporaryFile {
    path: PathBuf,
    target: PathBuf,
    committed: bool,
}

impl TemporaryFile {
    pub(super) fn new(target: &Path) -> io::Result<(Self, File)> {
        loop {
            let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
            let path = temporary_path(target, sequence);
            match OpenOptions::new().write(true).create_new(true).open(&path) {
                Ok(file) => {
                    return Ok((
                        Self {
                            path,
                            target: target.to_owned(),
                            committed: false,
                        },
                        file,
                    ));
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error),
            }
        }
    }

    pub(super) fn persist(mut self) -> io::Result<()> {
        fs::rename(&self.path, &self.target)?;
        self.committed = true;
        Ok(())
    }
}

impl Drop for TemporaryFile {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_file(&self.path);
        }
    }
}

fn temporary_path(target: &Path, sequence: u64) -> PathBuf {
    let name = target
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_default();
    target.with_file_name(format!(".{name}.tmp-{}-{sequence}", std::process::id()))
}
