use katoptron::Notification;
use crate::mirror;
use crate::cli;

use winapi::shared::minwindef::{UINT, WPARAM, LPARAM, LRESULT};
use winapi::shared::windef::{HWND};
use winapi::um::winnt::LPCWSTR;
use wstr_macro::wstr;
use crossbeam;
use crossbeam_channel::Sender;
use scopeguard;
use std::{mem, hint};
use lazy_static::{lazy_static, __lazy_static_internal, __lazy_static_create};


const SHELLHOOK_REG: LPCWSTR = wstr!["SHELLHOOK"];

static mut SENDER: Option<Sender<Notification>> = None;


unsafe fn init_message_sender(tx: Sender<Notification>) {
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
	use winapi::um::winuser::{DefWindowProcW, RegisterWindowMessageW, PostQuitMessage};
	use winapi::um::winuser::{GetWindowTextW, GetClassNameW};
	use winapi::um::winuser::{WM_DESTROY, HSHELL_WINDOWCREATED, HSHELL_FLASH};
	use winapi::ctypes::{c_int};
	use std::ops::Deref;
	use std::char;

	lazy_static! {
		static ref SHELLHOOK_VAL: UINT = unsafe {
			RegisterWindowMessageW(SHELLHOOK_REG)
		};
	}
	#[allow(non_snake_case)]
	let SHELLHOOK: UINT = *SHELLHOOK_VAL.deref();

	if msg == SHELLHOOK {
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

		return 0;
	}

	if msg == WM_DESTROY {
		PostQuitMessage(0);
		return 0;
	}

	DefWindowProcW(hwnd, msg, wparam, lparam)
}

//todo: use std::ptr::null

#[main]
fn main() {
	use std::mem;
	use winapi::um::winuser::{
		CreateWindowExW, RegisterShellHookWindow, SetWindowLongPtrW,
		GetMessageW, TranslateMessage, DispatchMessageW,
		HWND_MESSAGE, GWLP_WNDPROC,
		PostMessageW, WM_CLOSE,
	};
	use winapi::shared::basetsd::LONG_PTR;
	use winapi::ctypes::{c_void};

	//PostMessage() is safe to call from other threads
	struct Hwnd(winapi::shared::windef::HWND);
	unsafe impl Send for Hwnd {}

	let (server_address, _config_path) = cli::args();

	let window_handle = unsafe {
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
		Hwnd(window_handle)
	};

	crossbeam::scope(|scope| {
		let (sender, receiver) = crossbeam_channel::bounded(8);
		unsafe { init_message_sender(sender); }
		scope.defer(move || unsafe{ drop_message_sender() });

		scope.builder().name(String::from("sender")).spawn(move || {
			let _finally = scopeguard::guard((), move |_| unsafe { PostMessageW(window_handle.0, WM_CLOSE, 0, 0); });
			mirror::notifications(server_address, receiver);
		}).unwrap();

		unsafe {
			let mut msg = mem::zeroed();
			while GetMessageW(&mut msg, 0 as HWND, 0, 0) != 0 {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}
		}
	});
}
