extern crate crossbeam_channel;
extern crate crossbeam;
extern crate katoptron;

use self::crossbeam_channel::{Sender, Receiver};
use self::katoptron::Photon;
use std::{mem, hint};
use mirror;

static mut MSG_CHANNEL: (Option<Sender<Photon>>, Option<Receiver<Photon>>) = (None, None);

unsafe fn init_message_channel() {
	let (tx, rx) = crossbeam_channel::bounded(8);
	MSG_CHANNEL = (Some(tx), Some(rx));
}

unsafe fn message_sender() -> &'static Sender<Photon> {
	match MSG_CHANNEL.0 {
		Some(ref tx) => tx,
		_ => hint::unreachable_unchecked()
	}
}

unsafe fn message_receiver() -> Receiver<Photon> {
	match MSG_CHANNEL.1.take() {
		Some(rx) => rx,
		_ => hint::unreachable_unchecked()
	}
}

unsafe fn drop_message_channel() {
	match MSG_CHANNEL.0.take() {
		Some(tx) => mem::drop(tx),
		_ => hint::unreachable_unchecked()
	}
}

#[main]
fn main() {
	unsafe { init_message_channel(); }

	crossbeam::scope(|scope| {
		scope.builder().name(String::from("sender")).spawn(
			move || mirror::notifications(unsafe{ message_receiver() })
		).unwrap();
		scope.defer(move || unsafe{ drop_message_channel() });
//		scope.defer(move || unsafe{ PostMessage(window_handle, WM_CLOSE, 0, 0) });

		let notifications = unsafe{ message_sender() };

		let window_title = "Window Title";
		let window_class = "Window Class";
		notifications.send(Photon::Notification{ msg: format!("[created] {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		notifications.send(Photon::Flash{ msg: format!("[flashed] {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		notifications.send(Photon::Heartbeat).unwrap();
	});
}

