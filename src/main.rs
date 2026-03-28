mod argparse;
mod debounce;
mod watcher;

fn main() {
    let args = argparse::CliArgs::parse().unwrap_or_else(|e| {
        eprintln!("error: {e}");
        std::process::exit(1);
    });

    println!("path:      {}", args.path.display());
    println!("ext:       {:?}", args.ext);
    println!("debounce:  {}ms", args.debounce_ms);
    println!("command:   {:?}", args.command);
}
