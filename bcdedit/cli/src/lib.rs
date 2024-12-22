pub mod cli;
pub mod object;

mod utils;

use {
	bcdedit::{Store, StoreFlags},
	cli::{Cli, Ops},
	derive_more::{Display, Error},
	error_stack::{Result, ResultExt},
	owo_colors::OwoColorize,
	std::path::Path,
	tabled::Table,
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
	Store::create(&cli.store, StoreFlags::empty(), hivex::OpenFlags::empty())
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
	let objects = store.objects();
	if !descriptions {
		for object_h in objects.iter() {
			let object = store.object(*object_h).unwrap();
			if info {
				println!("{}", CustomDisplay(&object));
			} else {
				println!("{}", object.uuid());
			}
		}
	} else {
		#[derive(tabled::Tabled)]
		struct Row {
			#[tabled(rename = "Object UUID")]
			uuid: String,
			#[tabled(rename = "Description")]
			description: String,
		}

		let data = objects.iter().map(|object_h| {
			let object = store.object(*object_h).unwrap();
			let description = match object.get(bcdedit::elements::library::DESCRIPTION) {
				Ok(Some(value)) => CustomDisplay(&value).to_string(),
				Ok(None) => String::new(),
				Err(e) => e.red().italic().to_string(),
			};

			Row {
				uuid: object.uuid().to_string(),
				description,
			}
		});

		println!(
			"{}",
			Table::new(data).with(tabled::settings::Style::blank())
		);
	}

	Ok(())
}

fn open_store(path: impl AsRef<Path>, writable: bool) -> std::io::Result<Store> {
	let flags = if writable {
		hivex::OpenFlags::WRITE
	} else {
		hivex::OpenFlags::empty()
	};

	let hive = hivex::Hive::open(path, flags)?;
	Store::from_hive(hive).ok_or_else(|| {
		std::io::Error::new(
			std::io::ErrorKind::InvalidData,
			"Hive is not a valid BCD store",
		)
	})
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct CustomDisplay<'a, T>(&'a T);
