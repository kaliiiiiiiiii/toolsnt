use {
	crate::{cli::ObjectOps, open_store},
	bcdedit::object::typing::ObjectType,
	derive_more::{Display, Error},
	error_stack::{report, Result, ResultExt},
	owo_colors::OwoColorize,
	std::path::Path,
	tabled::settings::object::Rows,
	uuid::Uuid,
};

pub fn object(
	store_path: impl AsRef<Path>,
	uuid: Uuid,
	ops: Option<ObjectOps>,
) -> Result<(), ObjectManipulationError> {
	match ops {
		Some(_) => todo!(),
		None => infodump(store_path, uuid),
	}
}

fn infodump(store_path: impl AsRef<Path>, uuid: Uuid) -> Result<(), ObjectManipulationError> {
	let store = open_store(store_path, false).change_context(ObjectManipulationError::StoreOpen)?;
	let obj = store
		.object_lookup(uuid)
		.change_context(ObjectManipulationError::ObjectLookup)?
		.ok_or_else(|| {
			report!(ObjectManipulationError::ObjectLookup).attach_printable("Object was not found")
		})?;

	println!("{} {}", "Object:".bold(), uuid.as_hyphenated());
	print_object_type(obj.type_());
	println!("\n{}", "Elements:".underline().bold());

	// hack: Temporaty solution (they are often permanent)
	for (key, value) in obj.elements().with_values().flatten() {
		println!("{}{} {value:?}", key.bold(), ":".bold());
	}

	Ok(())
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum ObjectManipulationError {
	#[display("Failed to open store")]
	StoreOpen,
	#[display("Failed to lookup selected object")]
	ObjectLookup,
}

fn print_object_type(type_: ObjectType) {
	use bcdedit::object::typing::{ApplicationType, ImageType, InheritType};
	match type_ {
		ObjectType::Application { image, app_type } => {
			let image = match image {
				ImageType::Firmware => "Firmware",
				ImageType::WindowsBoot => "Windows Loader",
				ImageType::LegacyLoader => "NTLDR",
				ImageType::RealMode => "Real-mode",
			};

			let app_type = match app_type {
				ApplicationType::FwBootMgr => "Windows Boot Manager (UEFI)",
				ApplicationType::BootMgr => "Windows Boot Manager",
				ApplicationType::OsLoader => "Windows Boot Loader",
				ApplicationType::Resume => "Windows Resume Application",
				ApplicationType::MemDiag => "Windows Memory Tester",
				ApplicationType::NtLdr => "NTLDR",
				ApplicationType::SetupLdr => "Windows Setup",
				ApplicationType::BootSector => "Real-mode Application",
				ApplicationType::Startup => "Startup",
				ApplicationType::BootApp => "UEFI Application",
				ApplicationType::Unknown => "?",
			};

			println!(
				"{} Application\n├─ {} {image}\n╰─ {} {app_type}",
				"Type:".bold(),
				"Image Type:".bold(),
				"App Type:".bold()
			)
		}
		ObjectType::Inherit(inherit_type) => {
			let inherit_type = match inherit_type {
				InheritType::Any => "Any Object",
				InheritType::Application => "Applications",
				InheritType::Device => "Devices",
			};
			println!(
				"{} Inherit\n╰─ {} {inherit_type}",
				"Type:".bold(),
				"For:".bold()
			)
		}
		ObjectType::Device => println!("{} Device", "Type:".bold()),
	}
}
