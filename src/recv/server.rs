extern crate katoptron;
use self::katoptron::{Server, Connection, TxError, FailExt};

extern crate crossbeam;

extern crate crossbeam_channel;
use self::crossbeam_channel::Sender;

extern crate hostname;

use std::net::SocketAddr;


pub fn listen(flashes: Sender<String>) -> Result<(), TxError> {
	let name = hostname::get_hostname().unwrap_or_else(|| String::from("katoptron server"));
	let recv_addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let mut server = Server::listen_on(recv_addr, name)?;

	crossbeam::scope(|scope| {
		loop {
			match server.accept() {
				Ok((conn, client_name)) => {
					//todo: [someday] threadpool/async
					scope.builder().name(client_name.clone()).spawn({
						let flashes = flashes.clone();
						move || serve(conn, client_name, flashes)
					}).unwrap();
				}
				Err(e) => {
					eprintln!("{}", e.cause_trace());
				}
			};
		}
	});

	Ok(())
}

fn serve(mut conn: Connection, client_name: String, flashes: Sender<String>) {
	loop {
		use self::katoptron::Notification;
		match conn.recv_notification() {
			Ok(message) => match message {
				Notification::Popup{ msg } => {
					println!("Popup: {}", msg);
				},
				Notification::Flash{ msg } => {
					println!("Flash: {}", msg);
					flashes.send(msg);
				},
			},
			Err(e) => {
				eprintln!("{}: {}", client_name, e.cause_trace());
				break;
			}
		}
	}
}
