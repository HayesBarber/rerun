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
        let mut latest: Option<SystemTime> = None;

        if let Ok(entries) = fs::read_dir(&self.path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    let dir_latest = Self::dir_latest_mtime(&path, &self.extensions);
                    if let Some(t) = dir_latest {
                        latest = Some(latest.map_or(t, |curr| if t > curr { t } else { curr }));
                    }
                } else if self.matches_extension(&path) {
                    if let Ok(metadata) = fs::metadata(&path) {
                        if let Ok(modified) = metadata.modified() {
                            latest = Some(latest.map_or(modified, |curr| {
                                if modified > curr { modified } else { curr }
                            }));
                        }
                    }
                }
            }
        }

        latest
    }

    fn dir_latest_mtime(path: &Path, extensions: &[String]) -> Option<SystemTime> {
        let mut latest: Option<SystemTime> = None;

        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let entry_path = entry.path();
                if entry_path.is_dir() {
                    if let Some(t) = Self::dir_latest_mtime(&entry_path, extensions) {
                        latest = Some(latest.map_or(t, |curr| if t > curr { t } else { curr }));
                    }
                } else if extensions.is_empty()
                    || extensions.iter().any(|ext| {
                        entry_path
                            .extension()
                            .map_or(false, |e| e.to_string_lossy() == *ext)
                    })
                {
                    if let Ok(metadata) = fs::metadata(&entry_path) {
                        if let Ok(modified) = metadata.modified() {
                            latest = Some(latest.map_or(modified, |curr| {
                                if modified > curr { modified } else { curr }
                            }));
                        }
                    }
                }
            }
        }

        latest
    }

    fn matches_extension(&self, path: &Path) -> bool {
        if self.extensions.is_empty() {
            return true;
        }
        path.extension().map_or(false, |ext| {
            self.extensions.iter().any(|e| ext.to_string_lossy() == *e)
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
