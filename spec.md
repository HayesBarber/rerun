# Project Spec: `rerun` – Watch & Rerun CLI

**Goal:** Watch a directory tree for file changes and automatically restart a command.

---

## Features

1. **Recursive file watching**
   * MacOS: use `fsevent` crate
   * Secondary polling backend for testing/cross-platform

2. **CLI**
   * Flags:
     * `-p, --path <PATH>` → root directory (default `.`)
     * `-e, --ext <EXT>` → comma-separated extensions filter (optional)
     * `-d, --debounce <MS>` → quiet period in ms (default 200ms)
     * `-q, --quiet` → hide watcher output
   * `--` → separates CLI flags from the command to run
   * Command passed as raw arguments (everything after `--`)
   * No deps for this

3. **Debounce**
   * Global debounce: coalesce bursts of events into a single trigger
   * Default 200ms, configurable via CLI

4. **Child process management**
   * Spawn command with `std::process::Command`
   * On new trigger:
     * Send SIGINT (Unix/macOS) for graceful shutdown
     * Wait short timeout (200ms)
     * If still running, send SIGKILL
     * Restart command
   * Forward stdout/stderr to terminal
   * Clean exit on SIGINT

---

## Architecture

```text
src/
├── main.rs           # CLI parsing, channel setup, orchestration
├── watcher/
│   ├── mod.rs        # Watcher trait + re-exports
│   ├── fsevent.rs    # FSEventWatcher (macOS)
│   └── poll.rs       # Polling watcher backend
├── runner.rs         # Spawn/kill/restart child processes
├── debounce.rs       # Coalesce events into debounced triggers
└── argparse.rs       # Parse args and define usage
```

**Watcher trait:**

```rust
pub trait Watcher {
    fn run(&mut self, tx: std::sync::mpsc::Sender<std::path::PathBuf>);
}
```

* Decouples file watching from debounce and runner logic
* Allows swapping backend (FSEvents, polling, inotify, etc.)

---

## Event flow

```
File system event → Watcher → sends PathBuf to channel → Debounce → Trigger Runner → Kill/Restart Child → Forward output
```

---

## Constraints

* Weekend project
* Focus on macOS first
* Keep modules decoupled for later backend swaps

---

## Optional Stretch Goals

* Quiet mode
* Ignore common dirs (`.git`, `target`)
* Show which files changed
* Exit code propagation from child

