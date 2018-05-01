extern crate crossbeam_channel;
extern crate katoptron;
extern crate failure;

use std::net::{SocketAddr};
use self::katoptron::{Photon, Lightray, TxError};
use self::failure::Fail;

pub fn notifications(message_receiver: crossbeam_channel::Receiver<Photon>) {
	if let Err(e) = send_messages(message_receiver) {
		if let Some(backtrace) = e.backtrace() {
			println!("b; {}", backtrace);
		} else {
			println!("e; {}", e);
		}
	}
}

fn send_messages(message_receiver: crossbeam_channel::Receiver<Photon>) -> Result<(), TxError> {
	let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let mut lightray = Lightray::connect_to(&addr, String::from("ala ma kota — klient"))?;

	while let Ok(photon) = message_receiver.recv() {
		lightray.send_eavesdroppable_message(photon)?;
	}

	println!("Disperse!");
	lightray.disperse()?;
	Ok(())
}

//if let Some(bt) = err.cause().and_then(|cause| cause.backtrace()) {
//  println!("{}", bt)
//}
