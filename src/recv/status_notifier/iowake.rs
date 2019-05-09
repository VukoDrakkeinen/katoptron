use super::ffi::{cvt, cvt_r};
use byteorder::{ByteOrder, NativeEndian};
use libc::{self, c_void};
use std::os::unix::io::{RawFd, AsRawFd};
use std::{io, sync::Arc};

pub fn new() -> io::Result<(Wait, Wake)> {
	let fd = cvt(unsafe { libc::eventfd(0, libc::EFD_CLOEXEC | libc::EFD_NONBLOCK) })?;
	let fd = Arc::new(Fd { fd });

	Ok((Wait(fd.clone()), Wake(fd)))
}

#[derive(Clone)] pub struct Wait(Arc<Fd>);
#[derive(Clone)] pub struct Wake(Arc<Fd>);

impl Wait {
	pub fn ack(&self) -> u64 {
		let mut buf = [0u8; 8];

		let bufptr = buf.as_mut_ptr() as *mut c_void;
		cvt(unsafe { libc::read(self.0.fd, bufptr, buf.len()) }).unwrap();

		NativeEndian::read_u64(&buf)
	}

    #[allow(dead_code)]
	pub fn wait(&self) -> u64 {
		let mut pollfd: libc::pollfd = unsafe { std::mem::zeroed() };
		pollfd.fd = self.0.fd;
		pollfd.events = libc::POLLIN;

		cvt_r(|| unsafe { libc::poll(&mut pollfd as *mut _, 1, -1) }).unwrap();

		self.ack()
	}
}

impl Wake {
	pub fn wake(&self) {
		let mut buf = [0u8; 8];
		NativeEndian::write_u64(&mut buf, 1);

		let bufptr = buf.as_mut_ptr() as *mut c_void;
		cvt(unsafe { libc::write(self.0.fd, bufptr, buf.len()) }).unwrap();
	}
}

impl AsRawFd for Wait {
	fn as_raw_fd(&self) -> RawFd {
		self.0.fd
	}
}

struct Fd {
	fd: RawFd,
}

impl Drop for Fd {
	fn drop(&mut self) {
		let _ = unsafe { libc::close(self.fd) };
	}
}
