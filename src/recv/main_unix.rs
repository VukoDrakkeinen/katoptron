extern crate crossbeam;

extern crate katoptron;
use self::katoptron::FailExt;

extern crate crossbeam_channel;

extern crate clap;

extern crate dirs;

use crate::status_notifier;
use crate::server;

fn args() -> (u16) {
	use self::clap::{Error as ClapError, ErrorKind as ClapErrorKind};
	let arg_matches = clap_app!(("katoptron-recv") =>
		(version: crate_version!())
		(author: crate_authors!())
		(about: "Receives Windows events and shows them as freedesktop.org notifications on your Linux machine")
		(@arg port: -p --port +takes_value "Port to listen on")
	).get_matches();

	arg_matches
	.value_of("port")
	.map(
		|str_port|
		str_port
		.parse()
		.map_err(|_| ClapError::with_description(&format!("Port must be a number but '{}' was supplied", str_port), ClapErrorKind::ValueValidation))
		.and_then(|port| if port != 0 { Ok(port) } else { Err(ClapError::with_description("Port cannot be zero", ClapErrorKind::ValueValidation)) })
		.unwrap_or_else(|err| err.exit())
	)
	.unwrap_or(8888)
}

#[main]
fn main() {
	let port = args();
	crossbeam::scope(|scope| {
		let (flashes_sender, flasher_receiver) = crossbeam_channel::bounded(8);

		scope.builder().name(String::from("status_notifier")).spawn(
			move || status_notifier::show(flasher_receiver)
		).unwrap();

		if let Err(e) = server::listen(port, flashes_sender) {
			eprintln!("{}", e.cause_trace());
			//todo: exit with an error code
		}
	});
}
