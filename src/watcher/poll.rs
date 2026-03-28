use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;
use std::time::{Duration, SystemTime};

use super::Watcher;

pub struct PollWatcher {
    path: PathBuf,
    extensions: Vec<String>,
    poll_interval: Duration,
    last_mtime: Option<SystemTime>,
}

impl PollWatcher {
    pub fn new(path: PathBuf, extensions: Vec<String>, poll_interval: Duration) -> Self {
        Self {
            path,
            extensions,
            poll_interval,
            last_mtime: None,
        }
    }

    fn latest_mtime(&self) -> Option<SystemTime> {
        Self::dir_latest_mtime(&self.path, &self.extensions)
    }

    fn dir_latest_mtime(path: &Path, extensions: &[String]) -> Option<SystemTime> {
        let mut latest: Option<SystemTime> = None;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                let current = if entry_path.is_dir() {
                    Self::dir_latest_mtime(&entry_path, extensions)
                } else if Self::matches_extension(&entry_path, extensions) {
                    fs::metadata(&entry_path)
                        .ok()
                        .and_then(|m| m.modified().ok())
                } else {
                    None
                };

                if let Some(t) = current {
                    latest = Some(latest.map_or(t, |curr| if t > curr { t } else { curr }));
                }
            }
        }

        latest
    }

    fn matches_extension(path: &Path, extensions: &[String]) -> bool {
        extensions.is_empty()
            || path.extension().map_or(false, |ext| {
                extensions.iter().any(|e| ext.to_string_lossy() == *e)
            })
    }
}

impl Watcher for PollWatcher {
    fn run(&mut self, tx: Sender<()>) {
        self.last_mtime = self.latest_mtime();

        loop {
            std::thread::sleep(self.poll_interval);

            if let Some(current_mtime) = self.latest_mtime() {
                if Some(current_mtime) > self.last_mtime {
                    let _ = tx.send(());
                    self.last_mtime = Some(current_mtime);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::sync::mpsc;
    use std::thread;
    use tempfile::tempdir;

    #[test]
    fn matches_extension_empty_allows_all() {
        let ext = vec![];
        assert!(PollWatcher::matches_extension(&Path::new("foo.rs"), &ext));
        assert!(PollWatcher::matches_extension(&Path::new("bar.rs"), &ext));
        assert!(PollWatcher::matches_extension(&Path::new("foo.bar"), &ext));
    }

    #[test]
    fn matches_extension_filters() {
        let ext = vec!["rs".to_string()];
        assert!(PollWatcher::matches_extension(&Path::new("foo.rs"), &ext));
        assert!(!PollWatcher::matches_extension(&Path::new("foo.js"), &ext));
    }

    #[test]
    fn matches_extension_no_extension_fails() {
        let ext = vec!["rs".to_string()];
        assert!(!PollWatcher::matches_extension(
            &Path::new("Makefile"),
            &ext
        ));
    }

    #[test]
    fn dir_latest_mtime_empty_dir() {
        let dir = tempdir().unwrap();
        let path = dir.path();
        let result = PollWatcher::dir_latest_mtime(path, &[]);
        assert!(result.is_none());
    }

    #[test]
    fn dir_latest_mtime_single_file() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        File::create(&file_path).unwrap();
        let result = PollWatcher::dir_latest_mtime(dir.path(), &[]);
        assert!(result.is_some());
    }

    #[test]
    fn dir_latest_mtime_multiple_files() {
        let dir = tempdir().unwrap();
        let file1 = dir.path().join("a.rs");
        let file2 = dir.path().join("b.rs");
        File::create(&file1).unwrap();
        File::create(&file2).unwrap();
        let result = PollWatcher::dir_latest_mtime(dir.path(), &[]);
        assert!(result.is_some());
    }

    #[test]
    fn dir_latest_mtime_nested_dirs() {
        let dir = tempdir().unwrap();
        let subdir = dir.path().join("sub");
        fs::create_dir(&subdir).unwrap();
        let file = subdir.join("test.rs");
        File::create(&file).unwrap();
        let result = PollWatcher::dir_latest_mtime(dir.path(), &[]);
        assert!(result.is_some());
    }

    #[test]
    fn dir_latest_mtime_extension_filter() {
        let dir = tempdir().unwrap();
        let rs_file = dir.path().join("test.rs");
        let js_file = dir.path().join("test.js");
        File::create(&rs_file).unwrap();
        File::create(&js_file).unwrap();
        let ext = vec!["rs".to_string()];
        let result = PollWatcher::dir_latest_mtime(dir.path(), &ext);
        assert!(result.is_some());
    }

    #[test]
    fn run_detects_file_change() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        File::create(&file_path).unwrap();

        let (tx, rx) = mpsc::channel();
        let mut watcher =
            PollWatcher::new(dir.path().to_path_buf(), vec![], Duration::from_millis(50));

        thread::spawn(move || watcher.run(tx));

        thread::sleep(Duration::from_millis(100));
        File::create(&file_path).unwrap();

        let result = rx.recv_timeout(Duration::from_millis(200));
        assert!(result.is_ok(), "watcher should detect file change");
    }
}
