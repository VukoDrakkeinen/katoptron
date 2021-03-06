//todo(vuko): clean this mess up
mod poll;

use dbus;
use notify_rust::{Notification, NotificationHint, Timeout};
use crossbeam::channel::{self, select, Receiver};
use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::{rc::Rc, sync::Arc, cell::Cell};
use std::{mem, thread};
use poll::{IoReactor, WatchMode, IoEvents};

const FLASH_CUTOFF_INTERVAL:     Duration = Duration::from_secs(30);
const FLASH_EXPIRATION_INTERVAL: Duration = Duration::from_secs(600);

#[derive(Copy, Clone, Default, Debug)]
struct Data;
impl dbus::tree::DataType for Data {
	type Tree       = ();
	type ObjectPath = Rc<Cell<u16>>;
	type Property   = ();
	type Interface  = ();
	type Method     = ();
	type Signal     = ();
}

pub fn show(flashes: Receiver<String>) {
	let factory = dbus::tree::Factory::new_fnmut::<Data>();

	//todo: why are Arcs necessary here?
	let new_title_signal = Arc::new(factory.signal("NewTitle", ()));
	let new_icon_signal = Arc::new(factory.signal("NewIcon", ()));
	let new_attention_icon_signal = Arc::new(factory.signal("NewAttentionIcon", ()));
	let new_overlay_icon_signal = Arc::new(factory.signal("NewOverlayIcon", ()));
	let new_tooltip_signal = Arc::new(factory.signal("NewTooltip", ()));
	let new_status_signal = Arc::new(factory.signal("NewStatus", ()).sarg::<&str, _>("status"));

	//todo: can I get rid of Rc here somehow?
	let unread_notification_count = Rc::new(Cell::new(0));

	//IconPixmap(x: i32, y: i32, data: &[u8]), OverlayIconPixmap, AttentionIconPixmap, AttentionMovieName(name: &str), ToolTip(icon_name: &str, icon_data: (i32, i32, &[u8]), title: &str, description: &str)

	let tree = factory.tree(())
		.add(factory.object_path("/StatusNotifierItem", unread_notification_count.clone()).introspectable()
			.add(factory.interface("org.kde.StatusNotifierItem", ())
				.add_p(factory.property::<&str, _>("Category", ()).emits_changed(dbus::tree::EmitsChangedSignal::Const).on_get(|response, _| {
					response.append("Communications");
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("AttentionIconName", ()).emits_changed(dbus::tree::EmitsChangedSignal::Invalidates).access(dbus::tree::Access::Read).on_get(|response, _| {
					response.append("dialog-error");
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("IconName", ()).emits_changed(dbus::tree::EmitsChangedSignal::Invalidates).access(dbus::tree::Access::Read).on_get(|response, prop_info| {
					let unread_notifications_count = prop_info.path.get_data().get();
					println!("@icon c: {}", unread_notifications_count);
					if unread_notifications_count > 0 {
						response.append("dialog-error");
					} else {
						response.append("dialog-information");
					}
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("OverlayIconName", ()).emits_changed(dbus::tree::EmitsChangedSignal::Invalidates).access(dbus::tree::Access::Read).on_get(|response, _| {
					response.append("dialog-error");
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("Id", ()).emits_changed(dbus::tree::EmitsChangedSignal::Const).on_get(|response, _| {
					response.append("katoptron"); //todo: name of binary
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("Title", ()).emits_changed(dbus::tree::EmitsChangedSignal::Invalidates).access(dbus::tree::Access::Read).on_get(|response, _| {
					response.append("VM Notifications");
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("Status", ()).emits_changed(dbus::tree::EmitsChangedSignal::True).access(dbus::tree::Access::Read).on_get(|response, prop_info| {
					let unread_notification_count = prop_info.path.get_data().get();
					println!("@status c: {}", unread_notification_count);
					if unread_notification_count > 0 {
						response.append("NeedsAttention");
					} else {
						response.append("Active");
					}
					Ok(())
				}))
				.add_p(factory.property::<u32, _>("WindowId", ()).emits_changed(dbus::tree::EmitsChangedSignal::Const).on_get(|response, _| {
					response.append(0u32);
					Ok(())
				}))
				.add_p(factory.property::<bool, _>("ItemIsMenu", ()).emits_changed(dbus::tree::EmitsChangedSignal::Const).on_get(|response, _| {
					response.append(false);
					Ok(())
				}))
				.add_p(factory.property::<&str, _>("Menu", ()).emits_changed(dbus::tree::EmitsChangedSignal::Const).on_get(|response, _| {
					response.append("");
					Ok(())
				}))
				.add_m(factory.method("Activate", (), {
					let new_status_signal = new_status_signal.clone();
					let new_icon_signal = new_icon_signal.clone();

					move |call| {
						let _= call.msg.read2::<i32, i32>()?;

						let unread_notification_count = call.path.get_data();
						unread_notification_count.set(0);

						let ret = call.msg.method_return();
						let sig0 = new_status_signal.msg(call.path.get_name(), call.iface.get_name()).append1("Active");
						let sig1 = new_icon_signal.msg(call.path.get_name(), call.iface.get_name());
						Ok(vec![ret, sig0, sig1])
					}
				}).inarg::<i32, _>("x").inarg::<i32, _>("y"))
				.add_m(factory.method("ContextMenu", (), {
					let new_status_signal = new_status_signal.clone();
					let new_icon_signal = new_icon_signal.clone();

					move |call| {
						let _ = call.msg.read2::<i32, i32>()?;

						let unread_notification_count = call.path.get_data();
						unread_notification_count.set(1);

						let ret = call.msg.method_return();
						let sig0 = new_status_signal.msg(call.path.get_name(), call.iface.get_name()).append1("NeedsAttention");
						let sig1 = new_icon_signal.msg(call.path.get_name(), call.iface.get_name());
						Ok(vec![ret, sig0, sig1])
					}
				}).inarg::<i32, _>("x").inarg::<i32, _>("y"))
				.add_m(factory.method("SecondaryActivate", (), |call| {
					if let (Some(x), Some(y)) = call.msg.get2::<i32, i32>() {
						println!("Middle-clicked at ({x}, {y})", x=x, y=y);
					};
					Ok(vec![call.msg.method_return()])
				}).inarg::<i32, _>("x").inarg::<i32, _>("y"))
				.add_m(factory.method("Scroll", (), |call| {
					if let (Some(delta), Some(orientation)) = call.msg.get2::<i32, &str>() {
						println!("Scrolled {orientation}ly by {delta}", delta=delta, orientation=orientation);
					};
					Ok(vec![call.msg.method_return()])
				}).inarg::<i32, _>("delta").inarg::<&str, _>("orientation"))
				.add_s(new_title_signal)
				.add_s(new_icon_signal.clone())
				.add_s(new_attention_icon_signal)
				.add_s(new_overlay_icon_signal)
				.add_s(new_tooltip_signal)
				.add_s(new_status_signal.clone())
			)
		);

	let dbus_conn = dbus::Connection::get_private(dbus::BusType::Session).expect("couldn't connect to D-Bus");

	tree.set_registered(&dbus_conn, true).expect("couldn't register D-Bus paths");
	dbus_conn.add_handler(tree);

	let register = dbus::Message::new_method_call("org.kde.StatusNotifierWatcher", "/StatusNotifierWatcher", "org.kde.StatusNotifierWatcher", "RegisterStatusNotifierItem").expect("couldn't create message");
	let register = register.append(dbus_conn.unique_name());
	dbus_conn.send_with_reply_and_block(register, 250).expect("failed to register dbus name");

	let path  = dbus::Path::new("/StatusNotifierItem").unwrap();
	let iface = dbus::Interface::new("org.kde.StatusNotifierItem").unwrap();

    //We need to simultanously send & receive (process) data through dbus.
    //The simplest way would be to use 2 threads for that, but unfortunately
    //dbus::Connection is !Send + !Sync... IoReactor it is, then.
    let ioreactor = Arc::new(IoReactor::new().unwrap());

    dbus_conn.set_watch_callback(Box::new({
        let ioreactor = ioreactor.clone();
        move |dbus_fd| {
            let mode = WatchMode::from_bool_rw(dbus_fd.readable(), dbus_fd.writable());

            if mode == WatchMode::none() {
                ioreactor.unwatch(&dbus_fd).unwrap();
                return;
            }

            match ioreactor.change_mode(&dbus_fd, mode) {
                Ok(()) => return,
                Err(ref e) if e.kind() == std::io::ErrorKind::NotFound => {
                    ioreactor.watch(&dbus_fd, mode).unwrap();
                }
                Err(ref e) => panic!("failed to change dbus fd mode: {}", e),
            }
        }
    }));

    for dbus_fd in dbus_conn.watch_fds() {
        let mode = WatchMode::from_bool_rw(dbus_fd.readable(), dbus_fd.writable());
        if mode == WatchMode::none() { continue }
        ioreactor.watch(&dbus_fd, mode).unwrap();
    }

	let mut last_flash_times = HashMap::new();
	let past = Instant::now() - FLASH_CUTOFF_INTERVAL * 2; //too bad we can't use unix epoch here

    let (dbus_events_tx, dbus_events) = channel::bounded(0);

    thread::Builder::new().name("dbus-poll".to_owned()).spawn(move || {
        let mut ioevents = IoEvents::new();
        loop {
            ioreactor.wait(&mut ioevents).unwrap();

            for e in ioevents.drain() {
                if dbus_events_tx.send(e).is_err() {
                    return;
                }
            }
        }
    }).unwrap();

    loop {
        select! {
            recv(flashes) -> flash => {
                if flash.is_err() {
                    mem::drop(dbus_conn); //wake up the IoReactor thread
                    break;
                }

                let flash = flash.unwrap();
                let last_flash_time = last_flash_times.entry(flash.clone()).or_insert(past);

                //todo: [someday] anti-spam, n offences allowed, decays over time
                let now = Instant::now();
                if now - mem::replace(last_flash_time, now) <= FLASH_CUTOFF_INTERVAL {
                    continue;
                }
                last_flash_times.retain(move |_, &mut t| now - t <= FLASH_EXPIRATION_INTERVAL);

                unread_notification_count.modify(|c| *c += 1);

                dbus_conn.send(new_status_signal.msg(&path, &iface).append1("NeedsAttention")).unwrap();
                dbus_conn.send(new_icon_signal.msg(&path, &iface)).unwrap();

                //todo(vuko): drop the dep and send notifications ourselves through our conn
                Notification::new()
                    .summary("VM Notification")
                    .body(&flash)
                    .icon("emblem-shared")
                    .appname("katoptron")
                    .hint(NotificationHint::Category(String::from("message")))
                    .hint(NotificationHint::Resident(true)) //not supported by all implementations
                    .timeout(Timeout::Never) //this however is
                    .show().unwrap();
            }

            //non-blockingly process dbus (serve requests)
            recv(dbus_events) -> dbus_event => {
                let e = dbus_event.unwrap();

                dbus_conn.watch_handle(e.fd(), e.dbus_flags());
                for _ in (dbus::ConnMsgs { conn: &dbus_conn, timeout_ms: None }) {}
            }
        }
    }
}

trait CellModify<T: Copy> {
    fn modify<F>(&self, f: F) where F: for<'v> FnOnce(&'v mut T);
}

impl<T: Copy> CellModify<T> for Cell<T> {
    fn modify<F>(&self, f: F)
        where F: for<'v> FnOnce(&'v mut T)
    {
        let mut val = self.get();
        f(&mut val);
        self.set(val);
    }
}
