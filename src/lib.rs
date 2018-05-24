#![feature(nll)]
#![feature(box_syntax)]

extern crate byteorder;
use byteorder::{ByteOrder, LittleEndian, WriteBytesExt};

extern crate bincode;

extern crate serde;

#[macro_use]
extern crate serde_derive;

#[macro_use]
extern crate failure_derive;

#[macro_use]
extern crate display_derive;

extern crate failure;

use std::io::{self, Read, Write};
use std::net::{self, SocketAddr, TcpListener, TcpStream};

mod errors;
pub use errors::*;


const BUFFER_SIZE: usize     = 1024;
const MAGIC:           &[u8] = b"CAT";
const EAVESDROP_MAGIC: &[u8] = b"FACEBOOK_CAT";
const PROTOCOL_VERSION: u8 = 0;



//todo: leaky abstraction: ideally - client/server send/respond to heartbeats ON ITS OWN + handle mutliple clients (async I/O?)
//todo: better names: Photon -> Payload; new enum Message{ Notification, Flash, ?Heartbeat? }; Lightray -> NotificationStream
#[derive(Serialize, Deserialize)]
pub enum Photon {
	Handshake    { protocol_version: u8, peer_name: String },
	Heartbeat,
	Notification { msg: String },
	Flash        { msg: String },
}


pub struct Lightray {
	stream: TcpStream,
	buf: Box<[u8]>,
	peer_name: String,
}

impl Lightray {
	fn new(stream: TcpStream) -> Self {
		Self{ stream, buf: vec![0; BUFFER_SIZE].into_boxed_slice(), peer_name: String::new() }
	}

	fn send_handshake(&mut self, self_name: String) -> Result<(), SendError> {
		self.send_eavesdroppable_message(Photon::Handshake{ protocol_version: PROTOCOL_VERSION, peer_name: self_name })
	}

	//todo: weird flow control
	fn perform_handshake(&mut self, handshake: Handshake, self_name: String) -> Result<(), TxError> {
		if let Handshake::Initiate = handshake {
			self.send_handshake(self_name.clone())?; //stupid borrow-ck
		}

		if let Photon::Handshake{ protocol_version, peer_name } = self.recv_message()? {
			if let Handshake::Respond = handshake {
				self.send_handshake(self_name)?; //they're disjoint...
			}

			if protocol_version > PROTOCOL_VERSION {
				return Err(TxError::IncompatibleProtocol{ version: protocol_version });
			}

			self.peer_name = peer_name;
			return Ok(());
		}

		Err(TxError::HandshakeFailure)
	}

	pub fn listen_on(addr: SocketAddr, server_name: String) -> Result<Lightray, TxError> {
		let listener = TcpListener::bind(addr).with_context(|| "binding on (address here)")?;
		let (stream, send_addr) = listener.accept().with_context(|| "accepting connection from (address here)")?; //todo(vuko): handle multiple clients

		let mut lightray = Lightray::new(stream);
		lightray.perform_handshake(Handshake::Respond, server_name)?;
		Ok(lightray)
	}

	pub fn connect_to(addr: &SocketAddr, client_name: String) -> Result<Lightray, TxError> {
		use std::time::Duration;

		let stream = TcpStream::connect_timeout(&addr, Duration::from_secs(3)).with_context(|| "connecting to (address here)")?;

		let mut lightray = Lightray::new(stream);
		lightray.perform_handshake(Handshake::Initiate, client_name)?;
		Ok(lightray)
	}

	pub fn peer_name(&self) -> &str { &self.peer_name }

	//todo: timeouts
	pub fn recv_message(&mut self) -> Result<Photon, RecvError> {
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

	pub fn send_eavesdroppable_message<'a>(&mut self, payload: Photon) -> Result<(), SendError> {
		let payload_size = bincode::serialized_size(&payload)? as u16; //hmm, this basically serializes into a buffer that's then thrown away...?

		self.stream.write_all(&EAVESDROP_MAGIC).with_context(|| "sending message header")?;
		self.stream.write_u16::<LittleEndian>(payload_size).with_context(|| "sending message payload size")?;
		bincode::serialize_into(&mut self.stream, &payload)?;

		Ok(())
	}

	pub fn disperse(&mut self) -> Result<(), TxError> {
		self.stream.shutdown(net::Shutdown::Both).with_context(|| "disconnecting").map_err(|e| e.into())
	}
}

enum Handshake {
	Initiate,
	Respond,
}
