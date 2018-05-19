extern crate failure;
use failure::Fail;

extern crate bincode;

use std::io;
use std::fmt::{self, Display};
use std::convert::From;


#[derive(Fail, Debug, Display)]
pub enum TxError {
	#[display(fmt = "Handshake failure")]
	HandshakeFailure,

	#[display(fmt = "Incompatible protocol version {}", version)]
	IncompatibleProtocol{ version: u8 },

	#[display(fmt = "Transmission error")]
	Generic(#[fail(cause)] Box<Fail>, TxErrorKind),
}

impl TxError {
	pub fn kind(&self) -> TxErrorKind {
		match self {
			&TxError::HandshakeFailure         => return TxErrorKind::HandshakeFailure,
			&TxError::IncompatibleProtocol{..} => return TxErrorKind::IncompatibleProtocol,
			&TxError::Generic(_, kind)         => return kind,
		}
	}
}

#[derive(Debug, Display, Clone, Copy)]
pub enum TxErrorKind {
	HandshakeFailure,
	IncompatibleProtocol,
	Send,
	Recv,
	Io,
}


#[derive(Fail, Debug, Display)]
pub enum RecvError {
	#[display(fmt = "Payload too large")]
	PayloadTooLarge,

	#[display(fmt = "Payload deserialization failure")]
	PayloadDeserializationFailure(#[fail(cause)] bincode::Error),

	#[display(fmt = "Receive error")]
	Generic(#[fail(cause)] Box<Fail>, RecvErrorKind),
}

impl RecvError {
	pub fn kind(&self) -> RecvErrorKind {
		match self {
			&RecvError::PayloadTooLarge                  => return RecvErrorKind::PayloadTooLarge,
			&RecvError::PayloadDeserializationFailure(_) => return RecvErrorKind::PayloadDeserializationFailure,
			&RecvError::Generic(_, kind)                 => return kind,
		}
	}
}

#[derive(Debug, Display, Clone, Copy)]
pub enum RecvErrorKind {
	PayloadTooLarge,
	PayloadDeserializationFailure,
	Io,
}


#[derive(Fail, Debug, Display)]
pub enum SendError {
	#[display(fmt = "Payload serialization failure")]
	PayloadSerializationFailure {
		#[fail(cause)]
		cause: bincode::Error,
	},

	#[display(fmt = "Send error")]
	Generic(#[fail(cause)] Box<Fail>, SendErrorKind),
}

impl SendError {
	pub fn kind(&self) -> SendErrorKind {
		match self {
			&SendError::PayloadSerializationFailure{..} => return SendErrorKind::PayloadSerializationFailure,
			&SendError::Generic(_, kind)                => return kind,
		}
	}
}

#[derive(Debug, Display, Clone, Copy)]
pub enum SendErrorKind {
	PayloadSerializationFailure,
	Io,
}

#[derive(Fail, Display)]
#[display(fmt = "{}: {}", context, inner)]
pub struct ErrWithContext<E> {
	inner: E,
	context: Box<Display + Send + Sync + 'static>,
}

impl<E: Display + Send + Sync + 'static> fmt::Debug for ErrWithContext<E> {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}: {}", self.context, self.inner)
	}
}

pub trait ResultExt<T, E> {
//	fn with_context_val<C>(self, context: &C) -> Result<T, ErrWithContext<E>>
//	where
//		C: Display + Send + Sync + 'static;

	fn with_context<F, C>(self, f: F) -> Result<T, ErrWithContext<E>>
	where
		F: FnOnce() -> C,
		C: Display + Send + Sync + 'static;
}

impl<T> ResultExt<T, io::Error> for io::Result<T> {
//	fn with_context_val<C>(self, context: &C) -> Result<T, ErrWithContext<io::Error>>
//	where
//		C: Display + Send + Sync + 'static
//	{
//		self.map_err(|e| ErrWithContext{ cause: e, context: box context })
//	}

	fn with_context<F, C>(self, f: F) -> Result<T, ErrWithContext<io::Error>>
	where
		F: FnOnce() -> C,
		C: Display + Send + Sync + 'static,
	{
		self.map_err(|e| ErrWithContext{ inner: e, context: box f() })
	}
}


pub trait FailExt {
	fn cause_trace(&self) -> String;
}

impl<E: Fail> FailExt for E {
	fn cause_trace(&self) -> String {
		let mut trace = String::new();

		let mut current = self as &Fail;
		loop {
			if let Some(cause) = current.cause() {
				trace.push_str(&current.to_string());
				trace.push_str(", caused by\n");
				current = cause;
				continue;
			}
			break;
		}
		trace.push_str(&current.to_string());

		trace
	}
}


impl From<RecvError> for TxError {
	fn from(e: RecvError) -> Self {
		TxError::Generic(box e, TxErrorKind::Recv)
	}
}

impl From<SendError> for TxError {
	fn from(e: SendError) -> Self {
		TxError::Generic(box e, TxErrorKind::Send)
	}
}

//impl From<io::Error> for TxError {
//	fn from(e: io::Error) -> Self {
//		TxError::Generic(box e, TxErrorKind::Io)
//	}
//}

impl From<ErrWithContext<io::Error>> for TxError {
	fn from(e: ErrWithContext<io::Error>) -> Self {
		TxError::Generic(box e, TxErrorKind::Io)
	}
}


impl From<bincode::Error> for RecvError {
	fn from(e: bincode::Error) -> Self {
		RecvError::Generic(box e, RecvErrorKind::PayloadDeserializationFailure)
	}
}

//impl From<io::Error> for RecvError {
//	fn from(e: io::Error) -> Self {
//		RecvError::Generic(box e, RecvErrorKind::Io)
//	}
//}

impl From<ErrWithContext<io::Error>> for RecvError {
	fn from(e: ErrWithContext<io::Error>) -> Self {
		RecvError::Generic(box e, RecvErrorKind::Io)
	}
}


impl From<bincode::Error> for SendError {
	fn from(e: bincode::Error) -> Self {
		SendError::Generic(box e, SendErrorKind::PayloadSerializationFailure)
	}
}
impl From<ErrWithContext<io::Error>> for SendError {
	fn from(e: ErrWithContext<io::Error>) -> Self {
		SendError::Generic(box e, SendErrorKind::Io)
	}
}
