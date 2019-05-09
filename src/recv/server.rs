use katoptron::{Server, PreConnection, TxError, FailExt};

use crossbeam::{self, Sender};
use hostname;
use std::net::SocketAddr;
use crate::status_notifier::iowake::Wake;


pub fn listen(port: u16, flashes: Sender<String>, wake: Wake) -> Result<(), TxError> {
	let name = hostname::get_hostname().unwrap_or_else(|| String::from("katoptron server"));
	let recv_addr = SocketAddr::from(([0, 0, 0, 0], port));
	let mut server = Server::listen_on(recv_addr, name)?; //errors: UnableToBindAddress

	crossbeam::scope(|scope| {
		loop {
			match server.accept() { //errors: NetworkError <- IoError
				Ok(preconn) => {
					//todo: [someday] threadpool/async
					scope.builder().name(String::from("receiver")).spawn({
						let flashes = flashes.clone();
                        let wake = wake.clone();
						move |_| receive(preconn, flashes, wake)
					}).unwrap();
				}
				Err(e) => {
					eprintln!("{}", e.cause_trace());
				}
			};
		}
	}).unwrap();

	Ok(())
}

fn receive(preconn: PreConnection, flashes: Sender<String>, wake: Wake) {
    print!("Client connecting... ");
	let (mut conn, client_name) = match preconn.await_handshake() { //errors: HandshakeFailure <- GarbageData | ProtocolDowngrade
		Ok(ret) => ret,
		Err(e) => return eprintln!("{}", e.cause_trace()),
	};
    println!("ok");

	loop {
		use katoptron::Notification;
		match conn.recv_notification() { //errors: UnexpectedHandshake | [RecvError: NetworkError <- IoError+context | DeserializationError <- BincodeError | ProtocolDowngrade]
			Ok(message) => match message {
				Notification::Popup{ msg } => {
					println!("Popup: {}", msg);
				},
				Notification::Flash{ msg } => {
					println!("Flash: {}", msg);
					flashes.send(msg).unwrap();
                    wake.wake();
				},
			},
			Err(e) => {
				break eprintln!("{}: {}", client_name, e.cause_trace());
			}
		}
	}
}
