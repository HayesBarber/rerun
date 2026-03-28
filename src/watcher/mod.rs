use std::sync::mpsc::Sender;

#[cfg(target_os = "macos")]
pub mod fsevent;
pub mod poll;

pub trait Watcher {
    fn run(&mut self, tx: Sender<()>);
}
