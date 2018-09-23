use crossbeam_channel;
use crossbeam;
use katoptron;

use self::crossbeam_channel::Sender;
use self::katoptron::Notification;
use std::{mem, hint};
use crate::mirror;
use crate::cli;

static mut SENDER: Option<Sender<Notification>> = None;

unsafe fn init_message_channel(tx: Sender<Notification>) {
	SENDER = Some(tx);
}

unsafe fn message_sender() -> &'static Sender<Notification> {
	match SENDER {
		Some(ref tx) => tx,
		_ => hint::unreachable_unchecked(),
	}
}

unsafe fn drop_message_sender() {
	match SENDER.take() {
		Some(tx) => mem::drop(tx),
		_ => hint::unreachable_unchecked(),
	}
}

#[main]
fn main() {
	let (server_address, _config_path) = cli::args();
	let (tx, rx) = crossbeam_channel::bounded(8);
	unsafe { init_message_channel(tx); }

	crossbeam::scope(|scope| {
		scope.builder().name(String::from("sender")).spawn(
			move || mirror::notifications(server_address, rx)
		).unwrap();
		scope.defer(move || unsafe{ drop_message_sender() });
//		scope.defer(move || unsafe{ PostMessage(window_handle, WM_CLOSE, 0, 0) });

		let notifications = unsafe{ message_sender() };

		let window_title = "Window Title";
		let window_class = "Window Class";
		notifications.send(Notification::Popup{ msg: format!("[created] {title} {{{class}}}", title=window_title, class=window_class) });
		notifications.send(Notification::Flash{ msg: format!("[flashed]0 {title} {{{class}}}", title=window_title, class=window_class) });
		notifications.send(Notification::Flash{ msg: String::from("another flash") });
		notifications.send(Notification::Flash{ msg: format!("[flashed]1 {title} {{{class}}}", title=window_title, class=window_class) });
	});
}

