mod argparse;
mod debounce;
mod ignore;
mod runner;
mod watcher;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use ignore::IgnoreFilter;
use watcher::Watcher;

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

extern "C" fn handle_sigint(_sig: libc::c_int) {
    SHUTDOWN.store(true, Ordering::Relaxed);
}

fn main() {
    let args = argparse::CliArgs::parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let (watcher_tx, watcher_rx) = mpsc::channel();

    let ignore_filter = if args.no_ignore {
        IgnoreFilter::disabled()
    } else {
        IgnoreFilter::new(&args.path, &args.ignore, true)
    };

    let mut watcher: Box<dyn Watcher + Send> = {
        #[cfg(target_os = "macos")]
        {
            Box::new(watcher::fsevent::FSEventWatcher::new(
                args.path.clone(),
                args.ext.clone(),
                ignore_filter,
            ))
        }
        #[cfg(not(target_os = "macos"))]
        {
            let poll_interval = Duration::from_millis((args.debounce_ms / 2).max(50));
            Box::new(watcher::poll::PollWatcher::new(
                args.path.clone(),
                args.ext.clone(),
                poll_interval,
                ignore_filter,
            ))
        }
    };

    thread::spawn(move || watcher.run(watcher_tx));

    let debounce = debounce::Debounce::new(Duration::from_millis(args.debounce_ms), watcher_rx);

    let mut runner = runner::Runner::new();

    if let Err(e) = runner.spawn(&args.command) {
        eprintln!("error spawning command: {e}");
    }

    unsafe {
        libc::signal(
            libc::SIGINT,
            handle_sigint as *const () as libc::sighandler_t,
        );
    }

    while !SHUTDOWN.load(Ordering::Relaxed) {
        match debounce.debounced.recv_timeout(Duration::from_millis(100)) {
            Ok(()) => {
                eprintln!("\nchange detected, restarting...");
                if let Err(e) = runner.spawn(&args.command) {
                    eprintln!("error spawning command: {e}");
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    if SHUTDOWN.load(Ordering::Relaxed) {
        eprintln!("\nreceived SIGINT, shutting down...");
    }
}
