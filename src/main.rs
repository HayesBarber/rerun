mod argparse;
mod debounce;
mod runner;
mod watcher;

use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use watcher::Watcher;

fn main() {
    let args = argparse::CliArgs::parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    let (watcher_tx, watcher_rx) = mpsc::channel();

    let poll_interval = Duration::from_millis((args.debounce_ms / 2).max(50));
    let mut watcher =
        watcher::poll::PollWatcher::new(args.path.clone(), args.ext.clone(), poll_interval);

    thread::spawn(move || watcher.run(watcher_tx));

    let debounce = debounce::Debounce::new(Duration::from_millis(args.debounce_ms), watcher_rx);

    let mut runner = runner::Runner::new();

    if let Err(e) = runner.spawn(&args.command) {
        eprintln!("error spawning command: {e}");
    }

    while let Ok(()) = debounce.debounced.recv() {
        eprintln!("\nchange detected, restarting...");
        if let Err(e) = runner.spawn(&args.command) {
            eprintln!("error spawning command: {e}");
        }
    }
}
