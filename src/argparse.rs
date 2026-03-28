use std::env;
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub struct CliArgs {
    pub path: PathBuf,
    pub ext: Vec<String>,
    pub debounce_ms: u64,
    pub command: Vec<String>,
}

const USAGE: &str = "
█▀█ █▀▀ █▀█ █ █ █▄ █
█▀▄ ██▄ █▀▄ █▄█ █ ▀█

Usage: rerun [OPTIONS] -- <COMMAND>...

Watch a directory tree for file changes and automatically restart a command.

Options:
  -p, --path <PATH>       Root directory to watch (default: \".\")
  -e, --ext <EXT>         Comma-separated file extensions to filter (e.g. \"rs,toml\")
  -d, --debounce <MS>     Quiet period in milliseconds (default: 200)
  -h, --help              Print this help message

Arguments after \"--\" are treated as the command to run.";

impl CliArgs {
    pub fn parse() -> Result<Self, String> {
        Self::parse_from(env::args().skip(1))
    }

    fn parse_from(args: impl IntoIterator<Item = String>) -> Result<Self, String> {
        let mut path = PathBuf::from(".");
        let mut ext: Vec<String> = Vec::new();
        let mut debounce_ms: u64 = 200;
        let mut command: Vec<String> = Vec::new();

        let mut iter = args.into_iter();

        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "-h" | "--help" => {
                    eprintln!("{USAGE}");
                    std::process::exit(0);
                }
                "-p" | "--path" => {
                    let val = iter
                        .next()
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    path = PathBuf::from(val);
                }
                "-e" | "--ext" => {
                    let val = iter
                        .next()
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    ext = val.split(',').map(String::from).collect();
                }
                "-d" | "--debounce" => {
                    let val = iter
                        .next()
                        .ok_or_else(|| format!("missing value for {arg}"))?;
                    debounce_ms = val
                        .parse()
                        .map_err(|_| format!("invalid debounce value: {val}"))?;
                }
                "--" => {
                    command = iter.collect();
                    break;
                }
                other if other.starts_with('-') => {
                    return Err(format!("unknown option: {other}"));
                }
                _ => {
                    return Err(format!(
                        "unexpected argument: {arg}\nCommands must be passed after \"--\""
                    ));
                }
            }
        }

        if command.is_empty() {
            return Err("no command specified\n\nUsage: rerun [OPTIONS] -- <COMMAND>...".into());
        }

        Ok(Self {
            path,
            ext,
            debounce_ms,
            command,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn defaults() {
        let cli = CliArgs::parse_from(args(&["--", "echo", "hi"])).unwrap();
        assert_eq!(cli.path, PathBuf::from("."));
        assert!(cli.ext.is_empty());
        assert_eq!(cli.debounce_ms, 200);
        assert_eq!(cli.command, vec!["echo", "hi"]);
    }

    #[test]
    fn all_flags() {
        let cli = CliArgs::parse_from(args(&[
            "-p", "src", "-e", "rs,toml", "-d", "500", "--", "cargo", "test",
        ]))
        .unwrap();
        assert_eq!(cli.path, PathBuf::from("src"));
        assert_eq!(cli.ext, vec!["rs", "toml"]);
        assert_eq!(cli.debounce_ms, 500);
        assert_eq!(cli.command, vec!["cargo", "test"]);
    }

    #[test]
    fn long_flags() {
        let cli = CliArgs::parse_from(args(&[
            "--path",
            "lib",
            "--ext",
            "js,ts",
            "--debounce",
            "100",
            "--",
            "node",
            "index.js",
        ]))
        .unwrap();
        assert_eq!(cli.path, PathBuf::from("lib"));
        assert_eq!(cli.ext, vec!["js", "ts"]);
        assert_eq!(cli.debounce_ms, 100);
        assert_eq!(cli.command, vec!["node", "index.js"]);
    }

    #[test]
    fn no_command_errors() {
        let err = CliArgs::parse_from(args(&[])).unwrap_err();
        assert!(err.contains("no command specified"));
    }

    #[test]
    fn missing_flag_value_errors() {
        let err = CliArgs::parse_from(args(&["-p"])).unwrap_err();
        assert!(err.contains("missing value"));
    }

    #[test]
    fn unknown_flag_errors() {
        let err = CliArgs::parse_from(args(&["-x", "--", "ls"])).unwrap_err();
        assert!(err.contains("unknown option"));
    }

    #[test]
    fn bare_arg_errors() {
        let err = CliArgs::parse_from(args(&["foo", "--", "ls"])).unwrap_err();
        assert!(err.contains("unexpected argument"));
    }

    #[test]
    fn bad_debounce_errors() {
        let err = CliArgs::parse_from(args(&["-d", "abc", "--", "ls"])).unwrap_err();
        assert!(err.contains("invalid debounce value"));
    }

    #[test]
    fn single_ext() {
        let cli = CliArgs::parse_from(args(&["-e", "rs", "--", "cargo", "build"])).unwrap();
        assert_eq!(cli.ext, vec!["rs"]);
    }
}
