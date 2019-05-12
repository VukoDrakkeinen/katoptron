#![feature(main)]

mod server;

#[cfg(unix)] mod status_notifier;
#[cfg(unix)] mod main_unix;
#[cfg(windows)] fn main() { compile_error!("katoptron-recv is Unix-only"); }

//if we ever need an icon: must be a robocat; megatron <-> katoptron == mirror, get it? 8^)
