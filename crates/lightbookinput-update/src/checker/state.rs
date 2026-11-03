use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use lightbookinput_platform::UpdateChannel;

use super::Available;
use super::temporary::TemporaryFile;

/// 上次检查的结果，存在数据目录的 `update.json`；Windows 的设置程序直接读这个文件。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct UpdateState {
    /// 上次查成功的时间（Unix 秒）；0 = 还没查过。
    pub checked_at: u64,

    /// 那次查的是哪个渠道；换了渠道要立刻重查。
    pub channel: Option<UpdateChannel>,

    /// 那次查到的新版本。
    pub available: Option<Available>,
}

impl UpdateState {
    /// 读不到或读不懂都当没查过。
    pub fn load(path: &Path) -> Self {
        std::fs::read(path)
            .ok()
            .and_then(|bytes| serde_json::from_slice(&bytes).ok())
            .unwrap_or_default()
    }

    /// 先写临时文件再改名，读的一方不会看到半截。
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = serde_json::to_vec_pretty(self)?;
        let (temporary, mut file) = TemporaryFile::new(path)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
        drop(file);
        temporary.persist()
    }

    pub(crate) fn now() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |elapsed| elapsed.as_secs())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn scratch(test: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "lightbookinput-update-state-{}-{test}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn concurrent_saves_are_atomic_and_leave_no_temporary_files() {
        let dir = scratch("concurrent");
        let path = dir.join("update.json");
        let workers = 8;
        let barrier = Arc::new(Barrier::new(workers));
        let mut threads = Vec::new();
        for worker in 0..workers {
            let path = path.clone();
            let barrier = Arc::clone(&barrier);
            threads.push(thread::spawn(move || {
                barrier.wait();
                UpdateState {
                    checked_at: worker as u64 + 1,
                    ..UpdateState::default()
                }
                .save(&path)
            }));
        }
        for thread in threads {
            thread.join().unwrap().unwrap();
        }

        let state = UpdateState::load(&path);
        assert!((1..=workers as u64).contains(&state.checked_at));
        assert!(
            std::fs::read_dir(&dir)
                .unwrap()
                .filter_map(Result::ok)
                .all(|entry| !entry.file_name().to_string_lossy().contains(".tmp-"))
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
