#![feature(nll)]
#![feature(box_syntax)]

use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};
use bincode;
use serde_derive::{Serialize, Deserialize};
use std::io::{self, Read, Write};
use std::net::{self, SocketAddr, TcpListener, TcpStream};

mod errors;
pub use crate::errors::*;


const BUFFER_SIZE: usize     = 1024;
const MAGIC:           &[u8] = b"CAT";
const EAVESDROP_MAGIC: &[u8] = b"FACEBOOK_CAT";
const PROTOCOL_VERSION: u8 = 0;


#[derive(Serialize, Deserialize)]
enum Message {
	Handshake {
		protocol_version: u8,
		peer_name: String
	},
	Heartbeat,
	Notification(Notification),
}

#[derive(Serialize, Deserialize)]
pub enum Notification {
	Popup { msg: String },
	Flash { msg: String },
}

pub struct Server {
	listener: TcpListener,
	name: String,
}

impl Server {
	pub fn listen_on(addr: SocketAddr, server_name: String) -> Result<Server, TxError> {
		let listener = TcpListener::bind(addr).with_context(|| format!("binding on {}", addr))?;
		Ok(Server{ listener, name: server_name })
	}

	pub fn accept(&mut self) -> Result<PreConnection, TxError> {
		let (stream, _) = self.listener.accept().with_context(|| "accepting connection")?;
		Ok(PreConnection{ inner: Connection::new(stream), server_name: self.name.clone() })
	}
}

pub struct PreConnection {
	inner: Connection,
	server_name: String,
}

impl PreConnection {
	pub fn await_handshake(mut self) -> Result<(Connection, String), TxError> {
//		let client_addr = self.inner.stream.peer_addr().unwrap();
//		let client_name = self.inner.respond_to_handshake(self.server_name).with_context(|| format!("handshaking with {}", client_addr) )?; //todo
		let client_name = self.inner.respond_to_handshake(self.server_name)?;
		Ok((self.inner, client_name))
	}
}


pub struct Connection {
	stream: TcpStream,
	buf: Box<[u8]>,
}

impl Connection {
	fn new(stream: TcpStream) -> Self {
		Self{ stream, buf: vec![0; BUFFER_SIZE].into_boxed_slice() }
	}

	fn send_handshake(&mut self, self_name: String) -> Result<(), SendError> {
		self.send_eavesdroppable_message(Message::Handshake{ protocol_version: PROTOCOL_VERSION, peer_name: self_name })
	}

	fn initiate_handshake(&mut self, self_name: String) -> Result<String, TxError> {
		self.send_handshake(self_name)?;

		if let Message::Handshake{ protocol_version, peer_name } = self.recv_message()? {
			if protocol_version > PROTOCOL_VERSION {
				return Err(TxError::IncompatibleProtocol{ version: protocol_version });
			}

			return Ok(peer_name);
		}

		Err(TxError::HandshakeFailure)
	}

	//todo: should probably return HandshakeFailure if we receive garbage data
	fn respond_to_handshake(&mut self, self_name: String) -> Result<String, TxError> {
		if let Message::Handshake{ protocol_version, peer_name } = self.recv_message()? {
			self.send_handshake(self_name)?;

			if protocol_version > PROTOCOL_VERSION {
				return Err(TxError::IncompatibleProtocol{ version: protocol_version });
			}

			return Ok(peer_name);
		}

		Err(TxError::HandshakeFailure)
	}

	pub fn connect_to(addr: &SocketAddr, client_name: String) -> Result<(Connection, String), TxError> {
		use std::time::Duration;

		let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).with_context(|| "connecting to (address here)")?;

		let mut conn = Connection::new(stream);
		let server_name = conn.initiate_handshake(client_name)?;
		Ok((conn, server_name))
	}

	//todo: timeouts
	fn recv_message(&mut self) -> Result<Message, RecvError> {
		Self::recv_bytes(&mut self.stream, &mut self.buf[..EAVESDROP_MAGIC.len()]).with_context(|| "receiving message header")?;
		match &self.buf[..EAVESDROP_MAGIC.len()] {
			MAGIC => panic!("encryption not yet implemented"),
			EAVESDROP_MAGIC => { /*ok*/ },
			_ => return Err(RecvError::GarbageData)
		}

		let payload_size = {
			Self::recv_bytes(&mut self.stream, &mut self.buf[..2]).with_context(|| "receiving message payload size")?;
			//fuck RFC 1700
			LittleEndian::read_u16(&self.buf[..2]) as usize
		};
		if payload_size > BUFFER_SIZE {
			return Err(RecvError::PayloadTooLarge);
		}

		Self::recv_bytes(&mut self.stream, &mut self.buf[..payload_size]).with_context(|| "receiving message payload")?;
		bincode::deserialize(&self.buf).map_err(|e| e.into())
	}

	pub fn recv_notification(&mut self) -> Result<Notification, RecvError> {
		loop {
			match self.recv_message()? {
				Message::Notification(notification) => {
					return Ok(notification)
				},
				Message::Heartbeat => {
					continue;
				},
				Message::Handshake{..} => {
					//todo: better error (protocol error: unexpected handshake?)
					return Err(RecvError::GarbageData);
				},
			}
		}
	}

	fn recv_bytes(stream: &mut TcpStream, mut buf: &mut [u8]) -> Result<(), io::Error> {
		while !buf.is_empty() {
			match stream.read(buf) {
				Ok(0) => return Err(io::ErrorKind::ConnectionAborted.into()),
				Ok(n) => { buf = &mut buf[n..]; }
				Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
				Err(e) => return Err(e),
			}
		}

		Ok(())
	}

	fn send_eavesdroppable_message(&mut self, payload: Message) -> Result<(), SendError> {
		let payload_size = bincode::serialized_size(&payload)? as u16; //hmm, this basically serializes into a buffer that's then thrown away...?

		self.stream.write_all(&EAVESDROP_MAGIC).with_context(|| "sending message header")?;
		self.stream.write_u16::<LittleEndian>(payload_size).with_context(|| "sending message payload size")?;
		bincode::serialize_into(&mut self.stream, &payload)?;

		Ok(())
	}

	pub fn send_eavesdroppable_notification(&mut self, notification: Notification) -> Result<(), SendError> {
		self.send_eavesdroppable_message(Message::Notification(notification))
	}

	pub fn send_eavesdroppable_heartbeat(&mut self) -> Result<(), SendError> {
		self.send_eavesdroppable_message(Message::Heartbeat)
	}

	pub fn disconnect(&mut self) -> Result<(), TxError> {
		self.stream.shutdown(net::Shutdown::Both).with_context(|| "disconnecting").map_err(|e| e.into())
	}
}
