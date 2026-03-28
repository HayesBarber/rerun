use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub struct Debounce {
    pub debounced: mpsc::Receiver<()>,
    _handle: thread::JoinHandle<()>,
}

impl Debounce {
    pub fn new(duration: Duration, rx: mpsc::Receiver<std::path::PathBuf>) -> Self {
        let (tx, debounced) = mpsc::channel();

        let handle = thread::spawn(move || {
            Self::run(duration, &rx, &tx);
        });

        Self {
            debounced,
            _handle: handle,
        }
    }

    fn run(duration: Duration, rx: &mpsc::Receiver<std::path::PathBuf>, tx: &mpsc::Sender<()>) {
        let mut pending = false;

        loop {
            match rx.recv_timeout(duration) {
                Ok(_event) => {
                    pending = true;
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    if pending {
                        pending = false;
                        let _ = tx.send(());
                    }
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    if pending {
                        let _ = tx.send(());
                    }
                    break;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn send_paths(tx: &mpsc::Sender<PathBuf>, paths: &[&str]) {
        for p in paths {
            tx.send(PathBuf::from(p)).unwrap();
        }
    }

    #[test]
    fn single_event_triggers() {
        let (tx, rx) = mpsc::channel();
        let debounce = Debounce::new(Duration::from_millis(50), rx);

        send_paths(&tx, &["a.rs"]);
        drop(tx);

        assert!(debounce.debounced.recv().is_ok());
    }

    #[test]
    fn rapid_events_coalesce() {
        let (tx, rx) = mpsc::channel();
        let debounce = Debounce::new(Duration::from_millis(50), rx);

        for i in 0..20 {
            tx.send(PathBuf::from(format!("{i}.rs"))).unwrap();
            thread::sleep(Duration::from_millis(5));
        }
        drop(tx);

        // Should get exactly one trigger
        assert!(debounce.debounced.recv().is_ok());
        assert!(debounce.debounced.recv().is_err());
    }

    #[test]
    fn multiple_quiet_periods() {
        let (tx, rx) = mpsc::channel();
        let debounce = Debounce::new(Duration::from_millis(50), rx);

        // First burst
        send_paths(&tx, &["a.rs", "b.rs"]);
        thread::sleep(Duration::from_millis(100)); // long enough to trigger

        // Second burst
        send_paths(&tx, &["c.rs"]);
        thread::sleep(Duration::from_millis(100));

        drop(tx);

        // Two triggers expected
        assert!(debounce.debounced.recv().is_ok());
        assert!(debounce.debounced.recv().is_ok());
        assert!(debounce.debounced.recv().is_err());
    }

    #[test]
    fn no_spurious_triggers() {
        let (tx, rx) = mpsc::channel();
        let debounce = Debounce::new(Duration::from_millis(50), rx);

        // Wait several debounce periods with no events — should not trigger
        thread::sleep(Duration::from_millis(250));
        assert!(debounce.debounced.try_recv().is_err());

        // Now send an event, verify exactly one trigger
        send_paths(&tx, &["a.rs"]);
        thread::sleep(Duration::from_millis(100));
        assert!(debounce.debounced.recv().is_ok());
        assert!(debounce.debounced.try_recv().is_err());
    }

    #[test]
    fn input_closed_exits() {
        let (tx, rx) = mpsc::channel();
        let debounce = Debounce::new(Duration::from_millis(50), rx);

        drop(tx);

        // After the quiet period, the receiver should be disconnected
        thread::sleep(Duration::from_millis(100));
        assert!(debounce.debounced.recv().is_err());
    }
}
