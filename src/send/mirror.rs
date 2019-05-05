use katoptron::{Notification, Connection, TxError, FailExt};

use hostname;
use crossbeam_channel::{Receiver, select, __crossbeam_channel_parse, __crossbeam_channel_codegen};
use std::{net::SocketAddr, time::Duration};


pub fn notifications(server_address: SocketAddr, notification_receiver: Receiver<Notification>) -> i32 {
    let mut exit_code = 0;

	for _ in 0..3 {
		match send_messages(server_address, &notification_receiver) {
			Ok(()) => {
                exit_code = 0;
                break;
            },
			Err(e) => {
				eprintln!("{}", e.cause_trace());
                exit_code = 1;
			}
		}
	}

    exit_code
}

fn send_messages(server_address: SocketAddr, notification_receiver: &Receiver<Notification>) -> Result<(), TxError> {
	let name = hostname::get_hostname().unwrap_or_else(|| String::from("katoptron client"));
	let (mut conn, server_name) = Connection::connect_to(&server_address, name)?; //errors: HandshakeFailure <- GarbageData | ProtocolDowngrade | Timeout
	println!("Connected to server {} ({})", server_address, server_name);

	let timeout = Duration::from_millis(1000);
	loop {
		select! {
			recv(notification_receiver, notification) => {
				if let Some(notification) = notification {
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
