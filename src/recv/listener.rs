extern crate notify_rust;
use self::notify_rust::{Notification, NotificationHint, Timeout};

extern crate katoptron;
use self::katoptron::{Lightray, TxError, FailExt};

use std::net::SocketAddr;
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::mem;

const FLASH_CUTOFF_INTERVAL: Duration = Duration::from_secs(30);
const FLASH_EXPIRATION_INTERVAL: Duration = Duration::from_secs(600);

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
	let mut last_flash_times = HashMap::new();
	let past = Instant::now() - FLASH_CUTOFF_INTERVAL * 2;

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
					let now = Instant::now();
					let last_flash_time = last_flash_times.entry(msg.clone()).or_insert(past);

					//todo: [someday] anti-spam, n offences allowed, decays over time
					if now - mem::replace(last_flash_time, now) <= FLASH_CUTOFF_INTERVAL {
						continue;
					}

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

					expired_times_cleanup(&mut last_flash_times);
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

fn expired_times_cleanup(times: &mut HashMap<String, Instant>) {
	let now = Instant::now();
	times.retain(move |_, &mut time| now - time <= FLASH_EXPIRATION_INTERVAL);
}
