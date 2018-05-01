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

use std::convert::From;
use std::io::{self, Read, Write};
use std::net::{self, SocketAddr, TcpStream};

mod errors;
//pub use errors::*;


const BUFFER_SIZE: usize     = 1024;
//const MAGIC:           &[u8] = b"CAT";
const EAVESDROP_MAGIC: &[u8] = b"FACEBOOK_CAT";



//todo: better names
#[derive(Serialize, Deserialize)]
pub enum Photon {
	Handshake    { machine_name: String },
	Heartbeat,
	Notification { msg: String },
	Flash        { msg: String },
}


pub struct Lightray {
	stream: TcpStream,
	buf: Box<[u8]>,
	machine_name: String,
}

impl Lightray {
	pub fn new(stream: TcpStream) -> Self {
		Self{ stream, buf: vec![0; BUFFER_SIZE].into_boxed_slice(), machine_name: String::new() }
	}

	pub fn connect_to(address: &SocketAddr, client_name: String) -> Result<Lightray, CnctError> {
		use std::time::Duration;
//		use failure::ResultExt;
//		let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3)).context(address)?;
		let stream = TcpStream::connect_timeout(&address, Duration::from_secs(3))?;
		let mut lightray = Lightray::new(stream);

		lightray.send_eavesdroppable_message(Photon::Handshake{ machine_name: client_name })?;
		if let Photon::Handshake{ machine_name: server_name } = lightray.recv_message()? {
			lightray.machine_name = server_name;
			println!("Connected to {} at {}", lightray.machine_name, address);
			return Ok(lightray);
		}
		Err(CnctError::IncompatibleProtocolVersion(8))
	}

	pub fn recv_message(&mut self) -> Result<Photon, RecvError> {
		//errors: invalid header, invalid hash, connection closed,
		//header: 'FACEBOOK_CAT', version: byte, len: u16_LE, payload: Photon
		//header: 'CAT', version: byte, len: u16, payload: Photon

		//todo: timeouts
		{
			let mut infractions = 0;
			let mut nbytes_matched = 0;
			let nbytes_max = EAVESDROP_MAGIC.len();
			loop {
				Self::recv_bytes(&mut self.stream, &mut self.buf[nbytes_matched..nbytes_max])?;

				match find_magic(&self.buf[nbytes_matched..nbytes_max], &EAVESDROP_MAGIC[nbytes_matched..]) {
					MagicSearch::Found{ .. } => break,
					MagicSearch::NotFound => {
						infractions += 2;
						nbytes_matched = 0;
					},
					MagicSearch::ReadMore{ nbytes_discarded, .. } => {
						infractions += 1;
						nbytes_matched = nbytes_max - nbytes_discarded;
					},
				}

				if infractions > 8 {
					return Err(RecvError::CorruptedData(self.stream.peer_addr().unwrap()));
				}
			}
		}

		let version = {
			Self::recv_bytes(&mut self.stream, &mut self.buf[..1])?;
			self.buf[0]
		};
		if version != 0 {
			return Err(RecvError::IncompatibleProtocolVersion(version));
		}

		let payload_len = {
			Self::recv_bytes(&mut self.stream, &mut self.buf[..2])?;
			//fuck RFC 1700
			LittleEndian::read_u16(&self.buf[..2]) as usize
		};
		if payload_len > BUFFER_SIZE {
			return Err(RecvError::PayloadTooLarge);
		}

		Self::recv_bytes(&mut self.stream, &mut self.buf[..payload_len])?;
		bincode::deserialize(&self.buf).map_err(|e| e.into())
	}

	fn recv_bytes(stream: &mut TcpStream, mut buf: &mut [u8]) -> Result<(), io::Error> {
//		use io::ErrorKind::*;

		while !buf.is_empty() {
			match stream.read(buf) {
				Ok(0) => return Err(io::ErrorKind::ConnectionAborted.into()),
				Ok(n) => { buf = &mut buf[n..]; }
				Err(ref e) if e.kind() == io::ErrorKind::Interrupted => {}
				Err(e) => return Err(e),
			}
		}

		Ok(())

//		match stream.read_exact(&mut buf) {
//			Ok(()) => Ok(()),
//			Err(e) => match e.kind() {
//				ConnectionReset => panic!("conn reset"), //todo:
//				ConnectionAborted => Err(e),
//				WouldBlock | TimedOut => panic!("timeout"), //todo: do some other stuff, then retry (what? will vary)
//				UnexpectedEof if !once => { once = true; continue; }
//				_ => panic!(format!("what in the everliving fuck: {:?}", e.kind())),
//			}
//		}
	}

	pub fn send_eavesdroppable_message<'a>(&mut self, payload: Photon) -> Result<(), SendError> {
		let protocol_version = 0u8;
		let payload_size = bincode::serialized_size(&payload)? as u16; //hmm, this basically serializes into a buffer that's then thrown away...?

		self.stream.write_all(&EAVESDROP_MAGIC)?;
		self.stream.write_all(&[protocol_version])?;
		self.stream.write_u16::<LittleEndian>(payload_size)?;
		bincode::serialize_into(&mut self.stream, &payload)?;

		Ok(())
	}

	pub fn disperse(&mut self) -> Result<(), SendError> {
		self.stream.shutdown(net::Shutdown::Both).map_err(|e| e.into())
	}
}


#[derive(PartialEq, Eq, Debug)]
enum MagicSearch {
	Found { nbytes_discarded: usize },
	NotFound,
	ReadMore { nbytes_discarded: usize, nbytes_needed: usize },
}

fn find_magic(mut haystack: &[u8], magic: &[u8]) -> MagicSearch {
	if magic.is_empty() {
		return MagicSearch::Found{ nbytes_discarded: 0 };
	}

	//needle and haystack are probably too short to get fancy here (...Boyer-Moore?)
	let mut nbytes_discarded = 0;
	let mut nbytes_matched   = 0;
	'outer: while !haystack.is_empty() {
		for (haystack_probe, magic_probe) in haystack.iter().zip(magic.iter()) {
			if haystack_probe != magic_probe {
				haystack = &haystack[1..];
				nbytes_discarded += 1;
				nbytes_matched = 0;
				continue 'outer;
			}
			nbytes_matched += 1;
		}

		if magic.len() > haystack.len() {
			return MagicSearch::ReadMore{ nbytes_discarded, nbytes_needed: magic.len() - nbytes_matched };
		}

		if nbytes_matched == magic.len() {
			return MagicSearch::Found{ nbytes_discarded };
		}
	}

	MagicSearch::NotFound
}



//todo: holy boilerplate of batman!
#[derive(Fail, Debug, Display)]
pub enum RecvError {
	#[display(fmt = "RecvError: Corrupted data sent from {}", _0)]
	CorruptedData(SocketAddr),
	#[display(fmt = "RecvError: Incompatible protocol version {}", _0)]
	IncompatibleProtocolVersion(u8),
	#[display(fmt = "RecvError: Payload too large")]
	PayloadTooLarge,
	#[display(fmt = "RecvError: {}", _0)]
	Io(#[cause] io::Error),
	#[display(fmt = "RecvError: Payload deserialization failed ({})", _0)]
	DeserializationFailed(#[fail(cause)] bincode::Error)
}

impl From<io::Error> for RecvError {
	fn from(io: io::Error) -> Self {
		RecvError::Io(io)
	}
}

impl From<bincode::Error> for RecvError {
	fn from(de: bincode::Error) -> Self {
		RecvError::DeserializationFailed(de)
	}
}

#[derive(Fail, Debug, Display)]
pub enum SendError {
	#[display(fmt = "SendError: {}", _0)]
	Io(#[fail(cause)] io::Error),
	#[display(fmt = "SendError: Payload serialization failed ({})", _0)]
	SerializationFailed(#[cause] bincode::Error)
}

impl From<io::Error> for SendError {
	fn from(io: io::Error) -> Self {
		SendError::Io(io)
	}
}

impl From<bincode::Error> for SendError {
	fn from(de: bincode::Error) -> Self {
		SendError::SerializationFailed(de)
	}
}

#[derive(Fail, Debug, Display)]
pub enum CnctError {
	#[display(fmt = "CnctError: {}", _0)]
	Io(#[fail(cause)] io::Error),
	#[display(fmt = "CnctError: {}", _0)]
	Send(#[fail(cause)] SendError),
	#[display(fmt = "CnctError: {}", _0)]
	Recv(#[fail(cause)] RecvError),
	#[display(fmt = "CnctError: Incompatible protocol version {}", _0)]
	IncompatibleProtocolVersion(u8),
}

impl From<io::Error> for CnctError {
	fn from(io: io::Error) -> Self {
		CnctError::Io(io)
	}
}

impl From<SendError> for CnctError {
	fn from(send: SendError) -> Self {
		CnctError::Send(send)
	}
}

impl From<RecvError> for CnctError {
	fn from(recv: RecvError) -> Self {
		CnctError::Recv(recv)
	}
}


#[derive(Fail, Debug, Display)]
pub enum TxError {
	#[display(fmt = "TxError: {}", _0)]
	Send(#[fail(cause)] SendError),
	#[display(fmt = "TxError: {}", _0)]
	Recv(#[fail(cause)] RecvError),
	#[display(fmt = "TxError: {}", _0)]
	Cnct(#[fail(cause)] CnctError),
	#[display(fmt = "TxError: {}", _0)]
	Io(#[fail(cause)] io::Error),
}

impl From<CnctError> for TxError {
	fn from(cnct: CnctError) -> Self {
		TxError::Cnct(cnct)
	}
}

impl From<SendError> for TxError {
	fn from(send: SendError) -> Self {
		TxError::Send(send)
	}
}

impl From<RecvError> for TxError {
	fn from(recv: RecvError) -> Self {
		TxError::Recv(recv)
	}
}

impl From<io::Error> for TxError {
	fn from(io: io::Error) -> Self {
		TxError::Io(io)
	}
}
