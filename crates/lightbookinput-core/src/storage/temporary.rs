//! 同目录临时文件：并发写入各自使用独立文件，离开作用域时清理未提交的文件。

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
    /// 在目标同目录建一个独占的新文件；名字同时带进程号与进程内序号。
    pub(super) fn new(target: &Path, private: bool) -> io::Result<(Self, File)> {
        loop {
            let sequence = NEXT_TEMPORARY.fetch_add(1, Ordering::Relaxed);
            let path = temporary_path(target, sequence);
            let mut options = OpenOptions::new();
            options.write(true).create_new(true);
            #[cfg(unix)]
            if private {
                use std::os::unix::fs::OpenOptionsExt;
                options.mode(0o600);
            }
            #[cfg(not(unix))]
            let _ = private;
            match options.open(&path) {
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

    /// 改名覆盖目标。成功、失败都会由 `Drop` 保证不留下临时文件。
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
