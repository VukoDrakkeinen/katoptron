extern crate crossbeam;

extern crate katoptron;
use self::katoptron::FailExt;

extern crate crossbeam_channel;

use status_notifier;
use server;

#[main]
fn main() {
	crossbeam::scope(|scope| {
		let (flashes_sender, flasher_receiver) = crossbeam_channel::bounded(8);

		scope.builder().name(String::from("status_notifier")).spawn(
			move || status_notifier::show(flasher_receiver)
		).unwrap();

		if let Err(e) = server::listen(flashes_sender) {
			eprintln!("{}", e.cause_trace());
			//todo: exit with an error code
		}
	});
}
