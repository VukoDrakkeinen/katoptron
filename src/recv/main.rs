#![feature(nll)]
#![feature(main)]

mod listener;

#[cfg(unix)]
mod main_unix;
#[cfg(unix)]
mod status_notifier;

#[cfg(windows)]
fn main() { panic!("Unix only!"); }
