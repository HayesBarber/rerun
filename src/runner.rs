use std::io;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

pub struct Runner {
    child: Option<Child>,
}

impl Runner {
    pub fn new() -> Self {
        Self { child: None }
    }

    pub fn spawn(&mut self, command: &[String]) -> io::Result<()> {
        self.kill();

        let (program, args) = command
            .split_first()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "empty command"))?;

        let child = Command::new(program)
            .args(args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()?;

        self.child = Some(child);
        Ok(())
    }

    pub fn kill(&mut self) {
        if let Some(mut child) = self.child.take() {
            let pid = child.id() as i32;

            unsafe {
                libc::kill(pid, libc::SIGINT);
            }

            match child.wait_timeout(Duration::from_millis(200)) {
                Ok(Some(_)) => {
                    eprintln!("child process interrupted (pid {pid})");
                    return;
                }
                Ok(None) => {}
                Err(_) => {
                    eprintln!("child process interrupted (pid {pid})");
                    return;
                }
            }

            unsafe {
                libc::kill(pid, libc::SIGKILL);
            }

            let _ = child.wait();
            eprintln!("child process killed (pid {pid})");
        }
    }

    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.child.is_some()
    }
}

impl Drop for Runner {
    fn drop(&mut self) {
        self.kill();
    }
}

trait ChildExt {
    fn wait_timeout(&mut self, timeout: Duration) -> io::Result<Option<std::process::ExitStatus>>;
}

impl ChildExt for Child {
    fn wait_timeout(&mut self, timeout: Duration) -> io::Result<Option<std::process::ExitStatus>> {
        let start = std::time::Instant::now();
        loop {
            match self.try_wait() {
                Ok(Some(status)) => return Ok(Some(status)),
                Ok(None) => {
                    if start.elapsed() >= timeout {
                        return Ok(None);
                    }
                    thread::sleep(Duration::from_millis(10));
                }
                Err(e) => return Err(e),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn find_process(pid: i32) -> bool {
        let output = Command::new("ps")
            .args(["-p", &pid.to_string()])
            .output()
            .unwrap();
        output.status.success()
    }

    #[test]
    fn spawn_creates_child() {
        let mut runner = Runner::new();
        assert!(!runner.is_running());

        runner
            .spawn(&["sleep".into(), "60".into()])
            .expect("spawn should succeed");
        assert!(runner.is_running());

        runner.kill();
    }

    #[test]
    fn kill_stops_child() {
        let mut runner = Runner::new();
        runner
            .spawn(&["sleep".into(), "60".into()])
            .expect("spawn should succeed");

        runner.kill();
        assert!(!runner.is_running());
    }

    #[test]
    fn spawn_invalid_command_errors() {
        let mut runner = Runner::new();
        let result = runner.spawn(&["__no_such_binary_xyz__".into()]);
        assert!(result.is_err());
    }

    #[test]
    fn spawn_empty_command_errors() {
        let mut runner = Runner::new();
        let result = runner.spawn(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn spawn_replaces_previous() {
        let mut runner = Runner::new();
        runner
            .spawn(&["sleep".into(), "60".into()])
            .expect("first spawn should succeed");
        runner
            .spawn(&["sleep".into(), "60".into()])
            .expect("second spawn should succeed");
        assert!(runner.is_running());
        runner.kill();
    }

    #[test]
    fn kill_idempotent() {
        let mut runner = Runner::new();
        runner.kill();
        runner.kill();
        assert!(!runner.is_running());
    }

    #[test]
    fn drop_kills_child() {
        let pid = {
            let mut runner = Runner::new();
            runner
                .spawn(&["sleep".into(), "60".into()])
                .expect("spawn should succeed");
            let pid = runner.child.as_ref().unwrap().id() as i32;
            assert!(find_process(pid));
            pid
        };

        thread::sleep(Duration::from_millis(50));
        assert!(!find_process(pid), "process should be killed on drop");
    }

    #[test]
    fn child_exit_detected() {
        let mut runner = Runner::new();
        runner
            .spawn(&["true".into()])
            .expect("spawn should succeed");

        thread::sleep(Duration::from_millis(100));
        // process has exited but runner doesn't poll, so is_running stays true
        // until kill() is called. kill() should succeed without issue.
        runner.kill();
    }
}
