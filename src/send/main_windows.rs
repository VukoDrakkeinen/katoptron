use katoptron::Notification;
use crate::mirror;
use crate::cli;

use winapi::shared::minwindef::{UINT, WPARAM, LPARAM, LRESULT};
use winapi::shared::windef::{HWND};
use winapi::um::{winnt::LPCWSTR, winuser::RegisterWindowMessageW};
use wstr_macro::wstr;
use crossbeam::{self, channel, Sender};
use scopeguard;
use std::{mem, ptr, process, sync::atomic::{Ordering, AtomicI32}};


const SHELLHOOK_REG: LPCWSTR = wstr!["SHELLHOOK"];


struct WindowData {
	shellhook: UINT,
	notification_tx: Sender<Notification>,
	exit_code: AtomicI32,
}

impl WindowData {
	fn new(notification_tx: Sender<Notification>) -> *mut Self {
		Box::into_raw(box Self {
			shellhook: unsafe { RegisterWindowMessageW(SHELLHOOK_REG) },
			notification_tx,
			exit_code: AtomicI32::new(0),
		})
	}
}

unsafe extern "system"
fn wnd_proc(hwnd: HWND, msg_type: UINT, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
	use winapi::um::winuser::{DefWindowProcW, PostQuitMessage, DestroyWindow};
	use winapi::um::winuser::{GetWindowLongPtrW, GWLP_USERDATA};
	use winapi::um::winuser::{GetWindowTextW, GetClassNameW};
	use winapi::um::winuser::{WM_CLOSE, WM_DESTROY, WM_NCDESTROY, HSHELL_WINDOWCREATED, HSHELL_FLASH};
	use winapi::ctypes::{c_int};

	let window_data = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut WindowData;

	if msg_type == (*window_data).shellhook {
		let event_type    = wparam as i32;
		let window_handle = lparam as HWND;

		let mut title_buf = [0u16; 2048];
		let mut class_buf = [0u16; 2048];

		match event_type {
			HSHELL_WINDOWCREATED | HSHELL_FLASH => {
				let notification_tx = &(*window_data).notification_tx;

				let title_len = GetWindowTextW(window_handle, title_buf.as_mut_ptr(), title_buf.len() as c_int);
				let class_len = GetClassNameW( window_handle, class_buf.as_mut_ptr(), class_buf.len() as c_int);
                let window_title = String::from_utf16_lossy(&title_buf[..title_len as usize]);
                let window_class = String::from_utf16_lossy(&class_buf[..class_len as usize]);

                dbg!((&window_title, &window_class));

                let msg = format!("{} {{{}}}", window_title, window_class);
                if event_type == HSHELL_FLASH {
                    notification_tx.send(Notification::Flash{ msg }).unwrap();
                } else {
                    notification_tx.send(Notification::Popup{ msg }).unwrap();
                }
			},
			_ => {}
		}

		return 0;
	}

	if msg_type == WM_CLOSE {
		(*window_data).exit_code.store(lparam as i32, Ordering::Release);
		DestroyWindow(hwnd);
		return 0;
	}

	if msg_type == WM_DESTROY {
		let exit_code = (*window_data).exit_code.load(Ordering::Acquire);
		PostQuitMessage(exit_code);
		return 0;
	}

	if msg_type == WM_NCDESTROY {
		mem::drop(Box::from_raw(window_data));
		return 0;
	}

	DefWindowProcW(hwnd, msg_type, wparam, lparam)
}

#[main]
fn main() {
	let code = work();
	process::exit(code);
}

fn work() -> i32 {
	use winapi::um::winuser::{
		CreateWindowExW, RegisterShellHookWindow, SetWindowLongPtrW,
		GetMessageW, TranslateMessage, DispatchMessageW,
		HWND_MESSAGE, GWLP_WNDPROC, GWLP_USERDATA,
		PostMessageW, WM_CLOSE,
	};
	use winapi::shared::basetsd::LONG_PTR;

	//PostMessage() is safe to call from other threads
	struct Hwnd(winapi::shared::windef::HWND);
	unsafe impl Send for Hwnd {}
	impl Hwnd {
		unsafe fn close_window(self, exit_code: i32) { PostMessageW(self.0, WM_CLOSE, 0, exit_code as LPARAM); }
	}

	let (server_address, _config_path) = cli::args();

	let (notification_tx, notification_rx) = channel::bounded(8);
	let window_handle = unsafe {
		let window_handle = CreateWindowExW(
			/*style:*/ 0,
			/*class:*/ wstr!["Message"],
			/*title:*/ ptr::null(),
			/*style:*/ 0,
			/*x & y:*/ 0, 0,
			/*w & h:*/ 0, 0,
			/*parent*/ HWND_MESSAGE,
			/*menu :*/ ptr::null_mut(),
			/*instc:*/ ptr::null_mut(),
			/*lparam*/ ptr::null_mut(),
		);
        RegisterShellHookWindow(window_handle);
		SetWindowLongPtrW(window_handle, GWLP_USERDATA, WindowData::new(notification_tx) as LONG_PTR);
		SetWindowLongPtrW(window_handle, GWLP_WNDPROC, wnd_proc as LONG_PTR);
		Hwnd(window_handle)
	};

	crossbeam::scope(move |scope| {
		let panic_exit_code = 8;
		scope.builder().name("sender".into()).spawn(move |_| {
			let mut window_close_guard = scopeguard::guard(panic_exit_code, move |exit_code| unsafe { window_handle.close_window(exit_code); });
			let exit_code = &mut window_close_guard as &mut i32;
			*exit_code = mirror::notifications(server_address, notification_rx);
		}).unwrap();

		unsafe {
			let mut msg = mem::zeroed();
			while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) != 0 {
				TranslateMessage(&msg);
				DispatchMessageW(&msg);
			}
			msg.wParam as i32 //exit code
		}
	}).unwrap()
}
