#![feature(nll)]
#![feature(main)]
#![feature(proc_macro_hygiene)]

mod mirror;
mod cli;

#[cfg(windows)] mod main_windows;
//#[cfg(unix)] fn main() { compile_error!("katoptron-send is Windows-only"); }
#[cfg(unix)] mod proto_unix;
