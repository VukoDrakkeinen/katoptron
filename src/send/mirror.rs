extern crate crossbeam_channel;
extern crate katoptron;
extern crate hostname;
extern crate failure;

use crossbeam_channel::Receiver;
use self::katoptron::{Notification, Connection, TxError, FailExt};
use std::{net::SocketAddr, time::Duration};

pub fn notifications(notification_receiver: Receiver<Notification>) {
	if let Err(e) = send_messages(notification_receiver) {
		eprintln!("{}", e.cause_trace());
		//todo: exit error code
	}
}

fn send_messages(notification_receiver: Receiver<Notification>) -> Result<(), TxError> {
	let name = hostname::get_hostname().unwrap_or_else(|| String::from("katoptron client"));
	let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let (mut conn, server_name) = Connection::connect_to(&addr, name)?; //errors: HandshakeFailure <- GarbageData | ProtocolDowngrade | Timeout
	println!("Connected to server {} ({})", addr, server_name);

	let timeout = Duration::from_millis(1000);
	loop {
		select! {
			recv(notification_receiver, notification) => {
				if Some(notification) = notification {
					conn.send_eavesdroppable_notification(notification)?; //errors: SerializationFailure | PayloadTooLarge | NetworkError <- IoError
				} else {
					break;
				}
			},
			recv(crossbeam_channel::after(timeout)) => {
				conn.send_eavesdroppable_heartbeat()?; //errors: NetworkError <- IoError
			},
		}
	}

	conn.disconnect()?;
	Ok(())
}
