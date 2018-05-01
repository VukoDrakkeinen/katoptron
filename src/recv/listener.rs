extern crate notify_rust;
extern crate katoptron;
extern crate failure;

use std::net::{SocketAddr, TcpListener};
use self::notify_rust::{Notification, NotificationHint, Timeout};
use self::katoptron::{Lightray, TxError};

pub fn listen() {
	use self::failure::Fail;
	if let Err(e) = run_server() {
		if let Some(backtrace) = e.backtrace() {
			println!("{}", backtrace);
		} else {
			println!("{}", e);
		}
		//todo: exit with an error code
	}
}

fn run_server() -> Result<(), TxError> {
	let recv_addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let listener = TcpListener::bind(recv_addr)?;

	println!("listening...");

	let mut notification_count = 0;

	let (stream, send_addr) = listener.accept()?; //todo(vuko): handle multiple clients
	println!("connection from {}", send_addr);

	let mut lightray = Lightray::new(stream);
	loop {
		use self::katoptron::Photon;
		match lightray.recv_message() {
			Ok(photon) => match photon {
				Photon::Handshake{ machine_name: client_name } => {
					println!("Handshake from: {}", client_name);
					lightray.send_eavesdroppable_message(Photon::Handshake{ machine_name: String::from("ALA MA KOTA — serwer") })?;
				},
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
			},
			Err(e) => match e {
				_ => {
					println!("here: {}", e);
					break;
//					return Err(e.into());
				}
			}
		}
	}

	println!("{}", notification_count);
	Ok(())
}
