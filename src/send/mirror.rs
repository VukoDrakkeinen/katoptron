extern crate crossbeam_channel;
extern crate katoptron;
extern crate hostname;
extern crate failure;

use self::katoptron::{Notification, Connection, TxError, FailExt};
use std::{net::SocketAddr, time::Duration};

pub fn notifications(notification_receiver: crossbeam_channel::Receiver<Notification>) {
	if let Err(e) = send_messages(notification_receiver) {
		eprintln!("{}", e.cause_trace());
		//todo: exit error code
	}
}

fn send_messages(notification_receiver: crossbeam_channel::Receiver<Notification>) -> Result<(), TxError> {
	let name = hostname::get_hostname().unwrap_or_else(|| String::from("katoptron client"));
	let addr = SocketAddr::from(([192, 168, 122, 1], 8888));
	let (mut conn, server_name) = Connection::connect_to(&addr, name)?;
	println!("Connected to server {} ({})", addr, server_name);

	let timeout = Duration::from_millis(1000);
	loop {
		select! {
			recv(notification_receiver, notification) => {
				if notification.is_none() {
					break;
				}
				conn.send_eavesdroppable_notification(notification.unwrap())?;
			},
			recv(crossbeam_channel::after(timeout)) => {
				conn.send_eavesdroppable_heartbeat()?;
			},
		}
	}

	conn.disconnect()?;
	Ok(())
}
