extern crate notify_rust;
use self::notify_rust::{Notification, NotificationHint, Timeout};

extern crate katoptron;
use self::katoptron::{Lightray, TxError, FailExt};

use std::net::SocketAddr;

pub fn listen() {
	if let Err(e) = run_server() {
		eprintln!("{}", e.cause_trace());
		//todo: exit with an error code
	}
}

fn run_server() -> Result<(), TxError> {
	let recv_addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let mut lightray = Lightray::listen_on(recv_addr, String::from("ALA MA KOTA"))?;
	println!("Client connection {} ({})", recv_addr, lightray.peer_name());

	let mut notification_count = 0;

	loop {
		use self::katoptron::Photon;
		match lightray.recv_message() {
			Ok(photon) => match photon {
				Photon::Heartbeat => {
					println!("Heartbeat");
				},
				Photon::Notification{ msg } => {
					println!("Notification: {}", msg);
					notification_count += 1;
				},
				Photon::Flash{ msg } => {
					println!("Flash: {}", msg);
					Notification::new()
						.summary("VM Notification")
						.body(&msg)
						.icon("emblem-shared")
						.appname("katoptron")
						.hint(NotificationHint::Category(String::from("message")))
						.hint(NotificationHint::Resident(true)) // this is not supported by all implementations
						.timeout(Timeout::Never) // this however is
						.show().unwrap();
				},
				Photon::Handshake{..} => unreachable!()
			},
			Err(e) => match e {
				_ => {
					eprintln!("here: {}", e.cause_trace());
					break;
//					return Err(e.into());
				}
			}
		}
	}

	println!("notifs recv'd: {}", notification_count);
	Ok(())
}
