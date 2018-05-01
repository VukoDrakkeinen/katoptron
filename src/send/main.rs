#![feature(nll)]
#![feature(main)]
#![feature(unreachable)]

#[cfg(windows)]
#[macro_use] extern crate lazy_static;

//#[cfg(windows)]
mod mirror;
#[cfg(windows)]
mod main_windows;

//#[cfg(unix)]
//fn main() { panic!("Windows only!"); }

#[cfg(unix)]
mod proto_unix;
