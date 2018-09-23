#![feature(rust_2018_preview)]
#![feature(nll)]
#![feature(main)]
#![feature(proc_macro_non_items)]

#[cfg(windows)]
#[macro_use] extern crate lazy_static;

#[cfg(windows)]
extern crate wstr_macro;

#[macro_use]
extern crate crossbeam_channel;

#[cfg(windows)]
#[macro_use(defer)] extern crate scopeguard;

#[macro_use]
extern crate clap;

//#[cfg(windows)]
mod mirror;
mod cli;
#[cfg(windows)]
mod main_windows;

//#[cfg(unix)]
//fn main() { panic!("Windows only!"); }

#[cfg(unix)]
mod proto_unix;
