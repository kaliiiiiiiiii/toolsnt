pub mod cli;
pub mod object;

use {
	bcdedit::{Bcd, StoreFlags},
	cli::{Cli, Ops},
	derive_more::{Display, Error},
	error_stack::{Result, ResultExt},
	std::path::Path,
};

#[derive(Debug, Display, Error, PartialEq, Eq)]
pub enum ApplicationError {
	#[display("BCD initialization failed")]
	Init,
	#[display("Object data retrieval/manipulation error")]
	Object,
}

pub fn process_cli(cli: Cli) -> Result<(), ApplicationError> {
	match cli.ops {
		Ops::Init => init_bcd(&cli),
		Ops::Object { uuid, ops } => {
			object::object(cli.store, uuid, ops).change_context(ApplicationError::Object)
		}
	}
}

pub fn init_bcd(cli: &Cli) -> Result<(), ApplicationError> {
	Bcd::create(&cli.store, StoreFlags::empty(), hivex::OpenFlags::empty())
		.change_context(ApplicationError::Init)?;

	eprintln!("BCD Store {:?} initialized", &cli.store);
	Ok(())
}

fn open_store(path: impl AsRef<Path>, writable: bool) -> std::io::Result<Bcd> {
	let flags = if writable {
		hivex::OpenFlags::WRITE
	} else {
		hivex::OpenFlags::empty()
	};

	let hive = hivex::Hive::open(path, flags)?;
	Bcd::from_hive(hive).ok_or_else(|| {
		std::io::Error::new(
			std::io::ErrorKind::InvalidData,
			"Hive is not a valid BCD store",
		)
	})
}
