extern crate winapi;
use self::winapi::shared::minwindef::{UINT, WPARAM, LPARAM, LRESULT};
use self::winapi::shared::windef::{HWND};
use self::winapi::um::winnt::LPCWSTR;

extern crate wstr_macro;
use self::wstr_macro::wstr;

extern crate crossbeam;
extern crate crossbeam_channel;
use self::crossbeam_channel::Sender;

extern crate katoptron;
use self::katoptron::Notification;

use std::{mem, hint};
use mirror;


const SHELLHOOK_REG: LPCWSTR = wstr!["SHELLHOOK"];


static mut SENDER: Option<Sender<Notification>> = None;

unsafe fn init_sender(tx: Sender<Notification>) {
	SENDER = Some(tx);
}

unsafe fn message_sender() -> &'static Sender<Notification> {
	match SENDER {
		Some(ref tx) => tx,
		_ => hint::unreachable_unchecked(),
	}
}

unsafe fn drop_message_sender() {
	match SENDER.take() {
		Some(tx) => mem::drop(tx),
		_ => hint::unreachable_unchecked(),
	}
}

unsafe extern "system"
fn wnd_proc(hwnd: HWND, msg: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	use self::winapi::um::winuser::{DefWindowProcW, RegisterWindowMessageW, PostQuitMessage};
	use self::winapi::um::winuser::{GetWindowTextW, GetClassNameW};
	use self::winapi::um::winuser::{WM_DESTROY, HSHELL_WINDOWCREATED, HSHELL_FLASH};
	use self::winapi::ctypes::{c_int};
	use std::ops::Deref;
	use std::char;

	lazy_static! {
		static ref SHELLHOOK_VAL: UINT = unsafe {
			RegisterWindowMessageW(SHELLHOOK_REG)
		};
	}
	#[allow(non_snake_case)]
	let SHELLHOOK: UINT = *SHELLHOOK_VAL.deref();

	match msg {
		//the fuck does `SHELLHOOK =>` match everything?!
		_ if msg == SHELLHOOK => {
			let event_type = wparam as i32;
			let window_handle = lparam as HWND;

			let mut window_title = [0u16; 2048];
			let mut window_class = [0u16; 2048];

			match event_type {
				HSHELL_WINDOWCREATED => {
					GetWindowTextW(window_handle, window_title.as_mut_ptr(), window_title.len() as c_int);
					GetClassNameW( window_handle, window_class.as_mut_ptr(), window_class.len() as c_int);
					let window_title: String = char::decode_utf16(window_title.iter().cloned()).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).take_while(|c| *c != '\0').collect();
					let window_class: String = char::decode_utf16(window_class.iter().cloned()).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).take_while(|c| *c != '\0').collect();
					println!("{title} {{{class}}}", title=window_title, class=window_class);
					message_sender().send(Notification::Popup{ msg: format!("{title} {{{class}}}", title=window_title, class=window_class) });
				},

				HSHELL_FLASH => {
					GetWindowTextW(window_handle, window_title.as_mut_ptr(), window_title.len() as c_int);
					GetClassNameW( window_handle, window_class.as_mut_ptr(), window_class.len() as c_int);
					let window_title: String = char::decode_utf16(window_title.iter().cloned()).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).take_while(|c| *c != '\0').collect();
					let window_class: String = char::decode_utf16(window_class.iter().cloned()).map(|c| c.unwrap_or(char::REPLACEMENT_CHARACTER)).take_while(|c| *c != '\0').collect();
					println!("{title} {{{class}}}", title=window_title, class=window_class);
					message_sender().send(Notification::Flash{ msg: format!("{title} {{{class}}}", title=window_title, class=window_class) });
				},
				_ => {}
			}

			0
		},
		_ if msg == WM_DESTROY => {
//			message_sender().disconnect();
			PostQuitMessage(0);
			0
		},
		_ => {
			DefWindowProcW(hwnd, msg, wparam, lparam)
		}
	}
}

//todo: use std::ptr::null

#[main]
fn main() {
	use std::mem;
	use self::winapi::um::winuser::{CreateWindowExW, RegisterShellHookWindow, SetWindowLongPtrW};
	use self::winapi::um::winuser::{GetMessageW, TranslateMessage, DispatchMessageW};
	use self::winapi::um::winuser::{HWND_MESSAGE, GWLP_WNDPROC};
	use self::winapi::shared::basetsd::LONG_PTR;
	use self::winapi::ctypes::{c_void};

//	let mut window_handle;
	unsafe {
		let window_handle = CreateWindowExW(
			/*style:*/ 0,
			/*class:*/ wstr!["Message"],
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
	};
//	let window_handle = window_handle;

	let (sender, receiver) = crossbeam_channel::bounded(8);
	unsafe { init_sender(sender); }

	crossbeam::scope(|scope| {
		scope.builder().name(String::from("sender")).spawn(
			move || mirror::notifications(receiver)
		).unwrap();
		scope.defer(move || unsafe{ drop_message_sender() });
//		scope.defer(move || unsafe{ PostMessage(window_handle, WM_CLOSE, 0, 0) });

		unsafe {
			let mut msg = mem::zeroed();
			while GetMessageW(&mut msg, 0 as HWND, 0, 0) != 0 {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}
		}
	});
}