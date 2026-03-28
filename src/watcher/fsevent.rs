use std::path::{Path, PathBuf};
use std::sync::mpsc::Sender;

use fsevent::{Event, FsEvent, StreamFlags};

use super::Watcher;
use crate::ignore::IgnoreFilter;

pub struct FSEventWatcher {
    path: PathBuf,
    extensions: Vec<String>,
    ignore: IgnoreFilter,
}

impl FSEventWatcher {
    pub fn new(path: PathBuf, extensions: Vec<String>, ignore: IgnoreFilter) -> Self {
        Self {
            path,
            extensions,
            ignore,
        }
    }

    fn matches_extension(path: &Path, extensions: &[String]) -> bool {
        extensions.is_empty()
            || path.extension().map_or(false, |ext| {
                extensions.iter().any(|e| ext.to_string_lossy() == *e)
            })
    }
}

impl Watcher for FSEventWatcher {
    fn run(&mut self, tx: Sender<()>) {
        let path_str = self.path.to_string_lossy().to_string();
        let extensions = self.extensions.clone();
        let ignore = self.ignore.clone();

        let (event_tx, event_rx) = std::sync::mpsc::channel::<Event>();

        let fs_event = FsEvent::new(vec![path_str]);

        let filter_thread = std::thread::spawn(move || {
            let change_flags = StreamFlags::ITEM_CREATED
                | StreamFlags::ITEM_MODIFIED
                | StreamFlags::ITEM_REMOVED
                | StreamFlags::ITEM_RENAMED;

            while let Ok(event) = event_rx.recv() {
                if event.flag.intersects(StreamFlags::IS_DIR) {
                    continue;
                }
                if !event.flag.intersects(change_flags) {
                    continue;
                }
                let path = Path::new(&event.path);
                if ignore.is_ignored(path) {
                    continue;
                }
                if Self::matches_extension(path, &extensions) {
                    let _ = tx.send(());
                }
            }
        });

        fs_event.observe(event_tx);
        let _ = filter_thread.join();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::fs::File;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use tempfile::tempdir;

    fn no_ignore() -> IgnoreFilter {
        IgnoreFilter::disabled()
    }

    #[test]
    fn matches_extension_empty_allows_all() {
        let ext = vec![];
        assert!(FSEventWatcher::matches_extension(
            &Path::new("foo.rs"),
            &ext
        ));
        assert!(FSEventWatcher::matches_extension(
            &Path::new("bar.js"),
            &ext
        ));
    }

    #[test]
    fn matches_extension_filters() {
        let ext = vec!["rs".to_string()];
        assert!(FSEventWatcher::matches_extension(
            &Path::new("foo.rs"),
            &ext
        ));
        assert!(!FSEventWatcher::matches_extension(
            &Path::new("foo.js"),
            &ext
        ));
    }

    #[test]
    fn matches_extension_no_extension_fails() {
        let ext = vec!["rs".to_string()];
        assert!(!FSEventWatcher::matches_extension(
            &Path::new("Makefile"),
            &ext
        ));
    }

    #[test]
    fn detects_file_creation() {
        let dir = tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        let mut watcher = FSEventWatcher::new(dir.path().to_path_buf(), vec![], no_ignore());

        thread::spawn(move || watcher.run(tx));
        thread::sleep(Duration::from_millis(100));

        let file_path = dir.path().join("test.rs");
        File::create(&file_path).unwrap();

        let result = rx.recv_timeout(Duration::from_millis(2000));
        assert!(result.is_ok(), "watcher should detect file creation");
    }

    #[test]
    fn detects_file_modification() {
        let dir = tempdir().unwrap();
        let file_path = dir.path().join("test.rs");
        fs::write(&file_path, "initial").unwrap();

        let (tx, rx) = mpsc::channel();
        let mut watcher = FSEventWatcher::new(dir.path().to_path_buf(), vec![], no_ignore());

        thread::spawn(move || watcher.run(tx));
        thread::sleep(Duration::from_millis(100));

        fs::write(&file_path, "modified").unwrap();

        let result = rx.recv_timeout(Duration::from_millis(2000));
        assert!(result.is_ok(), "watcher should detect file modification");
    }

    #[test]
    fn extension_filter_ignores_non_matching() {
        let dir = tempdir().unwrap();
        let (tx, rx) = mpsc::channel();
        let mut watcher = FSEventWatcher::new(
            dir.path().to_path_buf(),
            vec!["rs".to_string()],
            no_ignore(),
        );

        thread::spawn(move || watcher.run(tx));
        thread::sleep(Duration::from_millis(100));

        fs::write(dir.path().join("test.js"), "hello").unwrap();

        let result = rx.recv_timeout(Duration::from_millis(500));
        assert!(
            result.is_err(),
            "watcher should ignore .js files when filtering for .rs"
        );
    }

    #[test]
    fn ignores_default_dirs() {
        let dir = tempdir().unwrap();
        let git_dir = dir.path().join(".git");
        fs::create_dir(&git_dir).unwrap();

        let ignore = IgnoreFilter::new(dir.path(), &[], false);
        let (tx, rx) = mpsc::channel();
        let mut watcher = FSEventWatcher::new(dir.path().to_path_buf(), vec![], ignore);

        thread::spawn(move || watcher.run(tx));
        thread::sleep(Duration::from_millis(100));

        fs::write(git_dir.join("HEAD"), "ref: refs/heads/main").unwrap();

        let result = rx.recv_timeout(Duration::from_millis(500));
        assert!(result.is_err(), "watcher should ignore files inside .git");
    }
}
