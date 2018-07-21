#![feature(nll)]
#![feature(main)]
#![feature(unreachable)]
#![feature(proc_macro)]
#![feature(proc_macro_non_items)]

#[cfg(windows)]
#[macro_use] extern crate lazy_static;

#[cfg(windows)]
extern crate wstr_macro;

#[macro_use]
extern crate crossbeam_channel;

#[cfg(windows)]
#[macro_use(defer)] extern crate scopeguard;

//#[cfg(windows)]
mod mirror;
#[cfg(windows)]
mod main_windows;

//#[cfg(unix)]
//fn main() { panic!("Windows only!"); }

#[cfg(unix)]
mod proto_unix;
