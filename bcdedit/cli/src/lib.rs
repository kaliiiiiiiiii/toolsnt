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
		Ops::List { info, descriptions } => list_objects(cli.store, info, descriptions),
	}
}

pub fn init_bcd(cli: &Cli) -> Result<(), ApplicationError> {
	Bcd::create(&cli.store, StoreFlags::empty(), hivex::OpenFlags::empty())
		.change_context(ApplicationError::Init)?;

	eprintln!("BCD Store {:?} initialized", &cli.store);
	Ok(())
}

pub fn list_objects(
	path: impl AsRef<Path>,
	info: bool,
	descriptions: bool,
) -> Result<(), ApplicationError> {
	let store = open_store(path, false).change_context(ApplicationError::Object)?;
	if !descriptions {
		for object_h in store.objects().iter() {
			let object = store.object(*object_h).unwrap();
			if info {
				println!("{}", CustomDisplay(&object));
			} else {
				println!("{}", object.uuid());
			}
		}
	} else {
		todo!("List with descriptions")
	}

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomDisplay<'a, T>(&'a T);
