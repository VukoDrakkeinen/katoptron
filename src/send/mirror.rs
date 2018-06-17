extern crate crossbeam_channel;
extern crate katoptron;
extern crate failure;

use std::net::{SocketAddr};
use self::katoptron::{Notification, Connection, TxError, FailExt};

pub fn notifications(notification_receiver: crossbeam_channel::Receiver<Notification>) {
	if let Err(e) = send_messages(notification_receiver) {
		eprintln!("{}", e.cause_trace());
		//todo: exit error code
	}
}

fn send_messages(notification_receiver: crossbeam_channel::Receiver<Notification>) -> Result<(), TxError> {
	let addr = SocketAddr::from(([127, 0, 0, 1], 8888));
	let (mut conn, server_name) = Connection::connect_to(&addr, String::from("client: ala ma kota"))?;
	println!("Connected to server {} ({})", addr, server_name);

	while let Some(notification) = notification_receiver.recv() {
		conn.send_eavesdroppable_notification(notification)?;
	}

	conn.disconnect()?;
	Ok(())
}
