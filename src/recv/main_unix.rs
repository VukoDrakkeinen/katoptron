use katoptron::FailExt;
use crate::status_notifier::{self, iowake};
use crate::server;

use crossbeam::{self, channel};
use clap::{clap_app, crate_version, crate_authors};


fn args() -> (u16) {
	use clap::{Error as ClapError, ErrorKind as ClapErrorKind};
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
		let (flashes_sender, flashes_receiver) = channel::bounded(8);
        let (wait, wake) = iowake::new().unwrap();

		scope.builder().name(String::from("status_notifier")).spawn(
			move |_| status_notifier::show(flashes_receiver, wait)
		).unwrap();

		if let Err(e) = server::listen(port, flashes_sender, wake) {
			eprintln!("{}", e.cause_trace());
			//todo: exit with an error code
		}
	}).unwrap();
}
