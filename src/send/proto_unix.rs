use katoptron::Notification;
use crate::mirror;
use crate::cli;

use crossbeam::{self, channel};
use std::{mem};

#[main]
fn main() {
	let (server_address, _config_path) = cli::args();
	let (tx, rx) = channel::bounded(8);

	crossbeam::scope(|scope| {
		scope.builder().name(String::from("sender")).spawn(
			move |_| mirror::notifications(server_address, rx)
		).unwrap();

		let window_title = "Window Title";
		let window_class = "Window Class";
		tx.send(Notification::Popup{ msg: format!("[created] {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		tx.send(Notification::Flash{ msg: format!("[flashed]0 {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		tx.send(Notification::Flash{ msg: String::from("another flash") }).unwrap();
		tx.send(Notification::Flash{ msg: format!("[flashed]1 {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
        mem::drop(tx);
	}).unwrap();
}

