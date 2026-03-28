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
                if let Some(last) = self.last_mtime {
                    if current_mtime > last {
                        let _ = tx.send(());
                        break;
                    }
                } else {
                    let _ = tx.send(());
                    break;
                }
            }
        }
    }
}
