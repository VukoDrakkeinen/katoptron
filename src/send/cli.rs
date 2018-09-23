use clap;
use dirs;

use std::{net::SocketAddr, path::PathBuf};

pub fn args() -> (SocketAddr, PathBuf) {
	use self::clap::{Error as ClapError, ErrorKind as ClapErrorKind};
	use std::fs::{DirBuilder, File};
	use std::path::Path;

	let matches = clap_app!(("katoptron-send") =>
		(version: crate_version!())
		(author: crate_authors!())
        (about: "Forwards Windows events as freedesktop.org notifications to your Linux machine")
        (@arg SERVER: +required "IPv4/6 of the remote machine receiving notifications")
        (@arg config: -c --config [FILE] "Custom config file path")
    ).get_matches();

	let server_address = {
		let str_addr = matches.value_of("SERVER").unwrap();
		str_addr
		.parse::<SocketAddr>()
		.map_err(|e| ClapError::with_description(&format!("'{}' is not a valid IPv4/6 address: {}", str_addr, e), ClapErrorKind::ValueValidation))
		.unwrap_or_else(|err| err.exit())
	};

	let config_dir = dirs::config_dir().expect("Unable to determine config directory");
	if !config_dir.is_dir() {
		DirBuilder::new()
		.create(&config_dir)
		.map_err(|e|
			ClapError::with_description(&format!("Cannot create config directory at '{}': {}", config_dir.display(), e), ClapErrorKind::ValueValidation)
		)
		.unwrap_or_else(|err| err.exit());
	}

	let config_path =
		if let Some(config) = matches.value_of("config") {
			let path = Path::new(config);
			if !path.is_file() {
				ClapError::with_description(&format!("No config at path '{}'", path.display()), ClapErrorKind::Io)
					.exit()
			}

			path.to_path_buf()
		} else {
			let config_ron = config_dir.join("config.ron");
			if config_ron.is_file() {
				config_ron
			} else if let Some(config_toml) = Some(config_dir.join("config.toml")).filter(|toml| toml.is_file()) {
				config_toml
			} else {
				File::create(&config_ron)
				.map_err(|err| ClapError::with_description(&format!("Unable to create config at '{}': {}", config_ron.display(), err), ClapErrorKind::Io))
				.unwrap_or_else(|err| err.exit());

				config_ron
			}
		};

	(server_address, config_path)
}
