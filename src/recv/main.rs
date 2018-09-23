#![feature(rust_2018_preview)]
#![feature(nll)]
#![feature(main)]

#[macro_use]
extern crate crossbeam_channel;

#[macro_use]
extern crate clap;

mod server;

#[cfg(unix)]
mod main_unix;
#[cfg(unix)]
mod status_notifier;

#[cfg(windows)]
fn main() { panic!("Unix only!"); }

//if we ever need an icon: must be a robocat; megatron <-> katoptron == mirror, get it? 8^)
