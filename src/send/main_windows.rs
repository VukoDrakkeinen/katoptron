#![feature(proc_macro)]
#![feature(proc_macro_non_items)]

extern crate winapi;
extern crate wstr_macro;
extern crate crossbeam_channel;
extern crate katoptron;

#[macro_use]
extern crate lazy_static;


use winapi::shared::minwindef::{UINT, WPARAM, LPARAM, LRESULT};
use winapi::shared::windef::{HWND};
use winapi::um::winnt::{LPCWSTR, LPWSTR};
use wstr_macro::wstr;
use crossbeam_channel;
use crossbeam_channel::{Sender, Receiver};
use stream;
use katoptron::Photon;


const SHELLHOOK_REG: LPCWSTR = wstr!["SHELLHOOK"];

lazy_static! {
	static (ref NOTIFICATIONS_TX, ref NOTIFICATIONS_RX): (Sender<_>, Receiver<_>) = crossbeam_channel::bounded::<Photon>(8);
}



unsafe extern "system"
fn wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	use winapi::um::winuser::{DefWindowProcW, RegisterWindowMessageW, PostQuitMessage};
	use winapi::um::winuser::{GetWindowTextW, GetClassNameW};
	use winapi::um::winuser::{HSHELL_WINDOWCREATED, HSHELL_FLASH};
	use winapi::ctypes::{c_int};
	use std::ops::Deref;

	lazy_static! {
		static ref SHELLHOOK_VAL: UINT = unsafe {
			RegisterWindowMessageW(SHELLHOOK_REG)
		};
	}
	let SHELLHOOK: UINT = *SHELLHOOK_VAL.deref();

	match msg {
		SHELLHOOK => {
			let event_type = wparam as i32;
			let window_handle = lparam as HWND;

			let mut window_title = [0u16; 2048];
			let mut window_class = [0u16; 2048];

			match event_type {
				HSHELL_WINDOWCREATED => {
					GetWindowTextW(window_handle, window_title.as_mut_ptr(), window_title.len() as c_int);
					GetClassNameW( window_handle, window_class.as_mut_ptr(), window_class.len() as c_int);
					let window_title = String::from_utf16_lossy(&window_title);
					let window_class = String::from_utf16_lossy(&window_class);
					println!("[created] {title} {{{class}}}", title=window_title, class=window_class);
					NOTIFICATIONS_TX.send(Photon::Notification{ msg: format!("[created] {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
				},

				HSHELL_FLASH => {
					GetWindowTextW(window_handle, window_title.as_mut_ptr(), window_title.len() as c_int);
					GetClassNameW( window_handle, window_class.as_mut_ptr(), window_class.len() as c_int);
					let window_title = String::from_utf16_lossy(&window_title);
					let window_class = String::from_utf16_lossy(&window_class);
					println!("[flashed] {title} {{{class}}}", title=window_title, class=window_class);
					NOTIFICATIONS_TX.send(Photon::Flash{ msg: format!("[flashed] {title} {{{class}}}", title=window_title, class=window_class) }).unwrap();
				},
				_ => {}
			}

			0
		},
		WM_DESTROY => {
			NOTIFICATIONS_TX.close();
			PostQuitMessage(0);
			0
		},
		_ => {
			DefWindowProcW(hwnd, msg, wparam, lparam)
		}
	}
}


#[main]
fn main() {
	use std::mem;
	use winapi::um::winuser::{CreateWindowExW, RegisterShellHookWindow, SetWindowLongPtrW};
	use winapi::um::winuser::{GetMessageW, TranslateMessage, DispatchMessageW};
	use winapi::um::winuser::{HWND_MESSAGE, GWLP_WNDPROC};
	use winapi::shared::basetsd::LONG_PTR;
	use winapi::ctypes::{c_void};
	use std::thread;

	NOTIFICATIONS_TX.send(Photon::Handshake{ machine_name: "ala ma kota" }).unwrap();

	let comms = thread::spawn(move || {
		stream::send_notifications(NOTIFICATIONS_RX);
	});

	unsafe {
		let window_handle = CreateWindowExW(
			/*style:*/ 0,
			/*class:*/ 0 as LPCWSTR,
			/*title:*/ 0 as LPCWSTR,
			/*style:*/ 0,
			/*x & y:*/ 0, 0,
			/*w & h:*/ 0, 0,
			/*parent*/ HWND_MESSAGE,
			/*menu :*/ 0 as *mut _,
			/*instc:*/ 0 as *mut _,
			/*lparam*/ 0 as *mut c_void,
		);
		RegisterShellHookWindow(window_handle);
		SetWindowLongPtrW(window_handle, GWLP_WNDPROC, wnd_proc as LONG_PTR);
	}

	unsafe {
		let mut msg = mem::zeroed();
		while GetMessageW(&mut msg, 0 as HWND, 0, 0) != 0 {
			TranslateMessage(&msg);
			DispatchMessageW(&msg);
		}
	}

	comms.join().unwrap();
}
