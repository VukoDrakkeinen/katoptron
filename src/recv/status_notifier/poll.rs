use super::ffi::cvt;
use libc::{self, c_int, c_uint};
use std::os::unix::io::{AsRawFd, RawFd};
use std::{io, ptr};

pub struct IoReactor {
	fd: RawFd,
}

impl Drop for IoReactor {
	fn drop(&mut self) {
		let _ = unsafe { libc::close(self.fd) };
	}
}

impl IoReactor {
	pub fn new() -> io::Result<Self> {
		let fd = cvt(unsafe { libc::epoll_create1(libc::EPOLL_CLOEXEC) })?;
		Ok(Self { fd })
	}

	pub fn watch(&self, watchable: &impl AsRawFd, mode: WatchMode) -> io::Result<()> {
		let fd = watchable.as_raw_fd();

		let mut cfg = libc::epoll_event {
			events: mode.0 as u32,
			u64:    fd as u64,
		};

		cvt(unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_ADD, fd, &mut cfg) })?;
		Ok(())
	}

	pub fn unwatch(&self, watchable: &impl AsRawFd) -> io::Result<()> {
		let fd = watchable.as_raw_fd();
		cvt(unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_DEL, fd, ptr::null_mut()) })?;
		Ok(())
	}

	pub fn change_mode(&self, watchable: &impl AsRawFd, mode: WatchMode) -> io::Result<()> {
		let fd = watchable.as_raw_fd();

		let mut cfg = libc::epoll_event {
			events: mode.0 as u32,
			u64:    fd as u64,
		};

		cvt(unsafe { libc::epoll_ctl(self.fd, libc::EPOLL_CTL_MOD, fd, &mut cfg) })?;
		Ok(())
	}

	pub fn wait(&self, events: &mut IoEvents) -> io::Result<()> {
		events.0.clear();

		let nready = cvt(unsafe { libc::epoll_wait(self.fd, events.0.as_mut_ptr(), events.0.capacity() as i32, -1) })?;
		unsafe {
			events.0.set_len(nready as usize);
		}

		Ok(())
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WatchMode(i32);

impl WatchMode {
	pub fn ro() -> Self {
		Self(libc::EPOLLIN)
	}
	pub fn wo() -> Self {
		Self(libc::EPOLLOUT)
	}
	pub fn rw() -> Self {
		Self(libc::EPOLLIN | libc::EPOLLOUT)
	}
	pub fn none() -> Self {
		Self(0)
	}
	pub fn from_bool_rw(read: bool, write: bool) -> Self {
		match (read, write) {
			(true, true) => Self::rw(),
			(true, false) => Self::ro(),
			(false, true) => Self::wo(),
			(false, false) => Self::none(),
		}
	}
}

pub struct IoEvents(Vec<libc::epoll_event>);

impl IoEvents {
	pub fn new() -> Self {
		Self(Vec::with_capacity(32))
	}
}

impl<'a> IntoIterator for &'a IoEvents {
	type Item = Event;
	type IntoIter = std::iter::Map<std::slice::Iter<'a, libc::epoll_event>, for<'r> fn(&'r libc::epoll_event) -> Event>;

	fn into_iter(self) -> Self::IntoIter {
		self.0.iter().map(Event::from_raw)
	}
}

pub struct Event {
	rawfd: RawFd,
	flags: c_int,
}

impl Event {
    pub fn relates_to(&self, watchable: &impl AsRawFd) -> bool {
        self.rawfd == watchable.as_raw_fd()
    }

	pub fn fd(&self) -> RawFd {
		self.rawfd
	}

	pub fn dbus_flags(&self) -> c_uint {
		let mut dbus_flags = 0;

		if self.flags & libc::EPOLLIN != 0 {
			dbus_flags |= dbus::WatchEvent::Readable as c_uint;
		}

		if self.flags & libc::EPOLLOUT != 0 {
			dbus_flags |= dbus::WatchEvent::Writable as c_uint;
		}

		if self.flags & libc::EPOLLERR != 0 {
			dbus_flags |= dbus::WatchEvent::Error as c_uint;
		}

		if self.flags & libc::EPOLLHUP != 0 {
			dbus_flags |= dbus::WatchEvent::Hangup as c_uint;
		}

		dbus_flags
	}

	fn from_raw(raw: &libc::epoll_event) -> Self {
		Self {
			rawfd: raw.u64 as RawFd,
			flags: raw.events as c_int,
		}
	}
}
