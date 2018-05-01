extern crate crossbeam;

use status_notifier;
use listener;

#[main]
fn main() {
	use std::sync::Arc;
	use std::sync::atomic::{AtomicBool, Ordering};

	let stop = Arc::new(AtomicBool::new(false));

	crossbeam::scope(|scope| {
		scope.builder().name(String::from("status_notifier")).spawn({
			let stop = stop.clone();
			move || status_notifier::show(stop)
		}).unwrap();
		scope.defer({
			//todo
//			let stop = stop.clone();
			move || stop.store(true, Ordering::SeqCst)
		});

		listener::listen();

//		//todo
//		stop.store(true, Ordering::SeqCst);
	});
}
