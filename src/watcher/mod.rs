use std::sync::mpsc::Sender;

pub mod poll;

pub trait Watcher {
    fn run(&mut self, tx: Sender<()>);
}
