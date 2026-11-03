//! 学习数据、配置这类小文件的落盘方式：先写同目录的临时文件，落盘后再改名覆盖。
//!
//! 输入法进程随时可能被 launchd 杀掉或崩溃；直接 `File::create` 覆盖写，中途断掉就留下半个文件，
//! 用户攒的词频 / n-gram 整个报废。改名在同一文件系统上是原子的，所以任何时刻磁盘上要么是旧文件、要么是新文件。

use std::fs::{self, File};
use std::io::{self, BufWriter, Write};
use std::path::Path;

mod temporary;

use temporary::TemporaryFile;

/// 原子地写 `path`：`write` 往缓冲写入端里写内容，写完 flush + fsync，再改名覆盖目标。
/// 中途任何一步失败都不碰目标文件，临时文件也清掉。
pub fn write_atomic(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    write_atomic_with(path, false, write)
}

/// 同 [`write_atomic`]，但文件只有本用户可读写（0600）：放密钥的 `.env` 用。
pub fn write_atomic_private(
    path: &Path,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    write_atomic_with(path, true, write)
}

fn write_atomic_with(
    path: &Path,
    private: bool,
    write: impl FnOnce(&mut BufWriter<File>) -> io::Result<()>,
) -> io::Result<()> {
    let (temporary, file) = TemporaryFile::new(path, private)?;
    (|| {
        let mut writer = BufWriter::new(file);
        write(&mut writer)?;
        writer.flush()?;
        writer.get_ref().sync_all()?;
        drop(writer);
        temporary.persist()
    })()
}

/// 把 `text` 原子地写进 `path`。
pub fn write_atomic_str(path: &Path, text: &str) -> io::Result<()> {
    write_atomic(path, |writer| writer.write_all(text.as_bytes()))
}

/// 读一个可能被写坏的文本文件：不存在返回 `None`；不是合法 UTF-8 的部分按替换字符读进来，让按行解析去跳过坏行，
/// 而不是整个文件读不了。其余 io 错误照常返回。
pub fn read_text_lossy(path: &Path) -> io::Result<Option<String>> {
    match fs::read(path) {
        Ok(bytes) => Ok(Some(String::from_utf8_lossy(&bytes).into_owned())),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};

    /// 每个测试一个目录：测试并行跑，共用目录时一个测试的临时文件会被另一个测试的目录扫描撞见
    fn scratch(test: &str, name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lightbookinput-storage-{}-{test}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir.join(name)
    }

    #[test]
    fn atomic_write_replaces_the_file_and_leaves_no_temporary() {
        let path = scratch("atomic", "atomic.tsv");
        write_atomic_str(&path, "old\n").unwrap();
        write_atomic_str(&path, "new\n").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "new\n");
        let leftovers: Vec<_> = fs::read_dir(path.parent().unwrap())
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp-"))
            .collect();
        assert!(leftovers.is_empty(), "临时文件没清：{leftovers:?}");
    }

    #[test]
    fn failed_write_keeps_the_old_file() {
        let path = scratch("kept", "kept.tsv");
        write_atomic_str(&path, "old\n").unwrap();
        let result = write_atomic(&path, |writer| {
            writer.write_all(b"half")?;
            Err(io::Error::other("boom"))
        });
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), "old\n");
        assert!(
            fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-"))
        );
    }

    #[test]
    fn concurrent_writes_use_separate_temporary_files() {
        let path = scratch("concurrent", "concurrent.tsv");
        let workers = 8;
        let barrier = Arc::new(Barrier::new(workers));
        let mut threads = Vec::new();
        for worker in 0..workers {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            threads.push(std::thread::spawn(move || {
                let text = format!("worker-{worker}\n");
                write_atomic(&path, |writer| {
                    // 所有写端都建好临时文件后再开始写，旧实现的共享路径会在这里互相截断。
                    barrier.wait();
                    writer.write_all(text.as_bytes())
                })
            }));
        }
        for thread in threads {
            thread.join().unwrap().unwrap();
        }
        let text = fs::read_to_string(&path).unwrap();
        assert!(text.starts_with("worker-"), "{text:?}");
        assert!(
            fs::read_dir(path.parent().unwrap())
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-"))
        );
    }

    #[cfg(unix)]
    #[test]
    fn private_write_is_owner_only() {
        use std::os::unix::fs::PermissionsExt;
        let path = scratch("private", "private.env");
        write_atomic_private(&path, |writer| writer.write_all(b"KEY=1\n")).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }

    #[test]
    fn lossy_read_tolerates_broken_utf8_and_missing_files() {
        let path = scratch("lossy", "broken.tsv");
        fs::write(&path, b"\xe5\xa5\xbd\t1\n\xe5\xa5\t2\n").unwrap();
        let text = read_text_lossy(&path).unwrap().unwrap();
        assert!(text.starts_with("好\t1\n"));
        assert!(text.contains('\u{FFFD}'));
        assert!(
            read_text_lossy(&scratch("lossy", "missing.tsv"))
                .unwrap()
                .is_none()
        );
    }
}
