#![feature(nll)]
#![feature(main)]

#[macro_use]
extern crate crossbeam_channel;

mod server;

#[cfg(unix)]
mod main_unix;
#[cfg(unix)]
mod status_notifier;

#[cfg(windows)]
fn main() { panic!("Unix only!"); }
