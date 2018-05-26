extern crate crossbeam_channel;
extern crate crossbeam;
extern crate katoptron;

use self::crossbeam_channel::Sender;
use self::katoptron::Photon;
use std::hint;
use mirror;

static mut SENDER: Option<Sender<Photon>> = None;

unsafe fn init_message_channel(tx: Sender<Photon>) {
	SENDER = Some(tx);
}

unsafe fn message_sender() -> &'static Sender<Photon> {
	match SENDER {
		Some(ref tx) => tx,
		_ => hint::unreachable_unchecked(),
	}
}

#[main]
fn main() {
	let (tx, rx) = crossbeam_channel::bounded(8);
	unsafe { init_message_channel(tx); }

	crossbeam::scope(|scope| {
		scope.builder().name(String::from("sender")).spawn(
			move || mirror::notifications(rx)
		).unwrap();
		scope.defer(move || unsafe{ message_sender().disconnect(); });
//		scope.defer(move || unsafe{ PostMessage(window_handle, WM_CLOSE, 0, 0) });

		let notifications = unsafe{ message_sender() };

		let window_title = "Window Title";
		let window_class = "Window Class";
		notifications.send(Photon::Notification{ msg: format!("[created] {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		notifications.send(Photon::Flash{ msg: format!("[flashed]0 {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		notifications.send(Photon::Flash{ msg: String::from("another flash") }).unwrap();
		notifications.send(Photon::Flash{ msg: format!("[flashed]1 {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
		notifications.send(Photon::Heartbeat).unwrap();
	});
}

