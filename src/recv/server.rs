extern crate katoptron;
use self::katoptron::{Lightray, TxError, FailExt};

extern crate crossbeam_channel;
use self::crossbeam_channel::Sender;

use std::net::SocketAddr;

pub fn listen(flashes: Sender<String>) -> Result<(), TxError> {
	let recv_addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let mut lightray = Lightray::listen_on(recv_addr, String::from("ALA MA KOTA"))?;
	println!("Client connection {} ({})", recv_addr, lightray.peer_name());

	loop {
		use self::katoptron::Photon;
		match lightray.recv_message() {
			Ok(photon) => match photon {
				Photon::Heartbeat => {
					println!("Heartbeat");
				},
				Photon::Notification{ msg } => {
					println!("Notification: {}", msg);
				},
				Photon::Flash{ msg } => {
					println!("Flash: {}", msg);
					flashes.send(msg).unwrap();
				},
				Photon::Handshake{..} => unreachable!()
			},
			Err(e) => match e {
				_ => {
					//todo
					eprintln!("here: {}", e.cause_trace());
					break;
//					return Err(e.into());
				}
			}
		}
	}

//	::std::thread::sleep_ms(10000);
	Ok(())
}
