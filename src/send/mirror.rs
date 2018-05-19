extern crate crossbeam_channel;
extern crate katoptron;
extern crate failure;

use std::net::{SocketAddr};
use self::katoptron::{Photon, Lightray, TxError, FailExt};

pub fn notifications(message_receiver: crossbeam_channel::Receiver<Photon>) {
	if let Err(e) = send_messages(message_receiver) {
		eprintln!("{}", e.cause_trace());
		//todo: exit error code
	}
}

fn send_messages(message_receiver: crossbeam_channel::Receiver<Photon>) -> Result<(), TxError> {
	let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let mut lightray = Lightray::connect_to(&addr, String::from("ala ma kota"))?;
	println!("Connected to server {} ({})", addr, lightray.peer_name());

	while let Ok(photon) = message_receiver.recv() {
		lightray.send_eavesdroppable_message(photon)?;
	}

	lightray.disperse()?;
	Ok(())
}
