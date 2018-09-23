use dbus;

use notify_rust;
use self::notify_rust::{Notification, NotificationHint, Timeout};

use crossbeam_channel;
use self::crossbeam_channel::Receiver;

use std::time::{Duration, Instant};
use std::collections::HashMap;
use std::cell::Cell;
use std::sync::Arc;
use std::rc::Rc;
use std::mem;

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
	if let Err(e) = dbus_conn.send_with_reply_and_block(register, 250) {
		println!("fuck: {}: {}", e.name().expect("b"), e.message().expect("c"));
	}

	let mut last_flash_times = HashMap::new();
	let past = Instant::now() - FLASH_CUTOFF_INTERVAL * 2; //I'd prefer unix epoch, but it's virtually impossible to get such Instant

	let path  = dbus::Path::new("/StatusNotifierItem").unwrap();
	let iface = dbus::Interface::new("org.kde.StatusNotifierItem").unwrap();

	let timeout = Duration::from_millis(50);
	loop {
		select! {
			recv(flashes, flash) => {
				if flash.is_none() {
					break;
				}
				let flash = flash.unwrap();

				let last_flash_time = last_flash_times.entry(flash).or_insert(past);

				//todo: [someday] anti-spam, n offences allowed, decays over time
				let now = Instant::now();
				if now - mem::replace(last_flash_time, now) <= FLASH_CUTOFF_INTERVAL {
					continue;
				}

				unread_notification_count.set(unread_notification_count.get() + 1);

				dbus_conn.send(new_status_signal.msg(&path, &iface).append1("NeedsAttention")).unwrap();
				dbus_conn.send(new_icon_signal.msg(&path, &iface)).unwrap();

				Notification::new()
					.summary("VM Notification")
					.body("ala ma kota")
					.icon("emblem-shared")
					.appname("katoptron")
					.hint(NotificationHint::Category(String::from("message")))
					.hint(NotificationHint::Resident(true)) // this is not supported by all implementations
					.timeout(Timeout::Never) // this however is
					.show().unwrap();

				last_flash_times.retain(|_, &mut time| Instant::now() - time <= FLASH_EXPIRATION_INTERVAL);
			},
			//unfortunately there seems no easy way of selecting on both channel and dbus
			recv(crossbeam_channel::after(timeout)) => for _ in dbus_conn.incoming(50) {}, //todo: why is the timeout in dbus necessary?
		}
	}
}
