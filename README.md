█▀█ █▀▀ █▀█ █ █ █▄ █<br>
█▀▄ ██▄ █▀▄ █▄█ █ ▀█

Watch a directory for file changes and automatically restart a command.

## Installation

```sh
brew install HayesBarber/tap/rerun
```

Or build from source:

```sh
cargo install --path .
```

## Usage

```
rerun [OPTIONS] -- <COMMAND>...

Options:
  -p, --path <PATH>       Root directory to watch (default: ".")
  -e, --ext <EXT>         Comma-separated file extensions to filter (e.g. "rs,toml")
  -d, --debounce <MS>     Quiet period in milliseconds (default: 200)
  -h, --help              Print this help message
```

Arguments after `--` are treated as the command to run.

## Examples

Rerun tests on any file change:

```sh
rerun -- cargo test
```

Watch only `.rs` and `.toml` files in `src/`:

```sh
rerun -p src -e rs,toml -- cargo build
```

Run a script with a 500ms debounce:

```sh
rerun -d 500 -- ./run.sh
```

## How it works

```
File system event → Watcher → Debounce → Runner → Kill/Restart child
```

- **Watcher** — uses native filesystem events on macOS ([`fsevent`](https://crates.io/crates/fsevent)) and falls back to polling on other platforms
- **Debounce** — coalesces bursts of changes into a single trigger (default 200ms)
- **Runner** — sends `SIGINT` for graceful shutdown, waits 200ms, then escalates to `SIGKILL` if needed, then respawns the command

Hitting `Ctrl-C` shuts down both `rerun` and the child process.

## License

MIT

