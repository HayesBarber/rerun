use std::collections::HashSet;
use std::fs;
use std::path::Path;

const DEFAULT_IGNORES: &[&str] = &[".git", "target", "node_modules", ".DS_Store", "__pycache__"];

#[derive(Clone)]
pub struct IgnoreFilter {
    patterns: HashSet<String>,
    disabled: bool,
}

impl IgnoreFilter {
    pub fn disabled() -> Self {
        Self {
            patterns: HashSet::new(),
            disabled: true,
        }
    }

    pub fn new(watch_root: &Path, cli_patterns: &[String], use_gitignore: bool) -> Self {
        let mut patterns: HashSet<String> = DEFAULT_IGNORES.iter().map(|s| s.to_string()).collect();

        for p in cli_patterns {
            let trimmed = p.trim();
            if !trimmed.is_empty() {
                patterns.insert(trimmed.to_string());
            }
        }

        if use_gitignore {
            let gitignore = watch_root.join(".gitignore");
            if let Ok(contents) = fs::read_to_string(&gitignore) {
                for line in contents.lines() {
                    let trimmed = line.trim();
                    if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with('!') {
                        continue;
                    }
                    let pattern = trimmed.trim_end_matches('/');
                    patterns.insert(pattern.to_string());
                }
            }
        }

        Self {
            patterns,
            disabled: false,
        }
    }

    pub fn is_ignored(&self, path: &Path) -> bool {
        if self.disabled {
            return false;
        }

        for component in path.components() {
            let name = component.as_os_str().to_string_lossy();
            for pattern in &self.patterns {
                if Self::matches(&name, pattern) {
                    return true;
                }
            }
        }

        false
    }

    fn matches(name: &str, pattern: &str) -> bool {
        if pattern.starts_with('*') {
            if let Some(suffix) = pattern.strip_prefix('*') {
                return name.ends_with(suffix);
            }
        }
        name == pattern
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn disabled_never_ignores() {
        let filter = IgnoreFilter::disabled();
        assert!(!filter.is_ignored(Path::new(".git/foo")));
        assert!(!filter.is_ignored(Path::new("target/debug/bar")));
    }

    #[test]
    fn default_patterns_present() {
        let dir = tempdir().unwrap();
        let filter = IgnoreFilter::new(dir.path(), &[], false);
        assert!(filter.is_ignored(Path::new(".git/config")));
        assert!(filter.is_ignored(Path::new("target/debug/app")));
        assert!(filter.is_ignored(Path::new("node_modules/foo")));
        assert!(filter.is_ignored(Path::new(".DS_Store")));
        assert!(filter.is_ignored(Path::new("__pycache__/mod.pyc")));
    }

    #[test]
    fn non_ignored_passes() {
        let dir = tempdir().unwrap();
        let filter = IgnoreFilter::new(dir.path(), &[], false);
        assert!(!filter.is_ignored(Path::new("src/main.rs")));
        assert!(!filter.is_ignored(Path::new("lib/mod.rs")));
    }

    #[test]
    fn nested_path_ignored() {
        let dir = tempdir().unwrap();
        let filter = IgnoreFilter::new(dir.path(), &[], false);
        assert!(filter.is_ignored(Path::new("project/.git/HEAD")));
        assert!(filter.is_ignored(Path::new("deep/nested/target/foo")));
    }

    #[test]
    fn cli_patterns_extend_defaults() {
        let dir = tempdir().unwrap();
        let cli = vec!["dist".to_string()];
        let filter = IgnoreFilter::new(dir.path(), &cli, false);
        assert!(filter.is_ignored(Path::new("dist/bundle.js")));
        assert!(filter.is_ignored(Path::new(".git/config")));
    }

    #[test]
    fn cli_comma_separated() {
        let dir = tempdir().unwrap();
        let cli = vec!["dist".to_string(), "out".to_string()];
        let filter = IgnoreFilter::new(dir.path(), &cli, false);
        assert!(filter.is_ignored(Path::new("dist/foo")));
        assert!(filter.is_ignored(Path::new("out/bar")));
    }

    #[test]
    fn glob_suffix_match() {
        let dir = tempdir().unwrap();
        let cli = vec!["*.log".to_string()];
        let filter = IgnoreFilter::new(dir.path(), &cli, false);
        assert!(filter.is_ignored(Path::new("error.log")));
        assert!(filter.is_ignored(Path::new("app/debug.log")));
        assert!(!filter.is_ignored(Path::new("main.rs")));
    }

    #[test]
    fn gitignore_parsed() {
        let dir = tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        fs::write(&gitignore, "# comment\n\nbuild\n!important/\n*.tmp\n").unwrap();

        let filter = IgnoreFilter::new(dir.path(), &[], true);
        assert!(filter.is_ignored(Path::new("build/output")));
        assert!(filter.is_ignored(Path::new("foo.tmp")));
        assert!(!filter.is_ignored(Path::new("important/keep")));
        assert!(!filter.is_ignored(Path::new("src/main.rs")));
    }

    #[test]
    fn gitignore_disabled() {
        let dir = tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        fs::write(&gitignore, "build\n").unwrap();

        let filter = IgnoreFilter::new(dir.path(), &[], false);
        assert!(!filter.is_ignored(Path::new("build/output")));
    }

    #[test]
    fn gitignore_trailing_slash_trimmed() {
        let dir = tempdir().unwrap();
        let gitignore = dir.path().join(".gitignore");
        fs::write(&gitignore, "build/\n").unwrap();

        let filter = IgnoreFilter::new(dir.path(), &[], true);
        assert!(filter.is_ignored(Path::new("build/output")));
    }

    #[test]
    fn no_gitignore_no_error() {
        let dir = tempdir().unwrap();
        let filter = IgnoreFilter::new(dir.path(), &[], true);
        assert!(!filter.is_ignored(Path::new("src/main.rs")));
    }
}
