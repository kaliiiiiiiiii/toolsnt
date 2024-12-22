use {
	crate::{
		cli::ObjectOps, open_store, utils::slice_try_for_each_interleaved_with_context,
		CustomDisplay,
	},
	bcdedit::{
		object::{elements_iter::PairsError, typing::ObjectType, Object},
		value::{
			device::{Device, Partition},
			format::DeviceFormat,
			GetValue, Value,
		},
	},
	derive_more::{Display, Error},
	error_stack::{report, Result, ResultExt},
	owo_colors::OwoColorize,
	std::{
		fmt::{Debug, Write},
		path::Path,
	},
	tabled::Table,
	uuid::Uuid,
};

pub fn object(
	store_path: impl AsRef<Path>,
	uuid: Uuid,
	ops: Option<ObjectOps>,
) -> Result<(), ObjectManipulationError> {
	match ops {
		Some(_) => todo!(),
		None => with_single_object(store_path, uuid, |obj| println!("{}", CustomDisplay(&obj))),
	}
}

pub fn with_single_object(
	store_path: impl AsRef<Path>,
	uuid: Uuid,
	mut f: impl FnMut(Object),
) -> Result<(), ObjectManipulationError> {
	let store = open_store(store_path, false).change_context(ObjectManipulationError::StoreOpen)?;
	let obj = store
		.object_lookup(uuid)
		.change_context(ObjectManipulationError::ObjectLookup)?
		.ok_or_else(|| {
			report!(ObjectManipulationError::ObjectLookup).attach_printable("Object was not found")
		})?;

	f(obj);
	Ok(())
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum ObjectManipulationError {
	#[display("Failed to open store")]
	StoreOpen,
	#[display("Failed to lookup selected object")]
	ObjectLookup,
	#[display("Underlying object operation")]
	Ops,
}

impl Display for CustomDisplay<'_, Object<'_>> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let type_ = self.0.type_();

		// Print header and object info
		writeln!(
			f,
			"{}{}\n{}{}\n\n{}",
			"Object: ".bold().underline(),
			self.0.uuid().bold().underline(),
			"Type: ".bold(),
			CustomDisplay(&type_),
			"Elements:".bold(),
		)?;

		let elements = self.0.elements();
		let data = elements.with_values().map(|result| match result {
			Ok((k, v)) => (k.display(type_).to_string(), CustomDisplay(&v).to_string()),
			Err(report) => match report.current_context() {
				// Failed to get element's name => Unknown Element (0xHEX)
				PairsError::NameParse { name } => (
					format!(
						"{}{}{}",
						"Unknown element (\"".italic(),
						name.italic(),
						"\")".italic()
					),
					String::new(),
				),
				// Failed to decode value => Error
				PairsError::ValueDecode { element } => (
					element.display(type_).to_string(),
					report.red().italic().to_string(),
				),
			},
		});

		let mut elements_table = Table::new(data);
		let elements_table = elements_table.with(tabled::settings::Style::blank()).with(
			tabled::settings::Disable::row(tabled::settings::object::Rows::first()),
		);

		writeln!(f, "{elements_table}")?;

		Ok(())
	}
}

impl Display for CustomDisplay<'_, ObjectType> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use bcdedit::object::typing::{ApplicationType, ImageType, InheritType};
		match self.0 {
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

				write!(
					f,
					"Application\n├─ {} {image}\n╰─ {} {app_type}",
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
				write!(f, "Inherit\n╰─ {} {inherit_type}", "For:".bold())
			}
			ObjectType::Device => f.write_str("Device"),
		}
	}
}

impl Display for CustomDisplay<'_, GetValue<'_>> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match &self.0 {
			Value::Device(dev) => Display::fmt(&CustomDisplay(dev), f),
			Value::String(string) => f.write_str(string),
			Value::Guid(uuid) => Display::fmt(&uuid.braced(), f),
			Value::GuidList(uuids) => slice_try_for_each_interleaved_with_context(
				uuids,
				f,
				|f, uuid| Display::fmt(&uuid.braced(), f),
				|f| f.write_char('\n'),
			),
			Value::Integer(int) => Display::fmt(int, f),
			Value::Bool(boolean) => Display::fmt(boolean, f),
			Value::IntegerList(list) => slice_try_for_each_interleaved_with_context(
				list,
				f,
				|f, n| write!(f, "{n:#x}"),
				|f| f.write_str(", "),
			),
		}
	}
}

impl Display for CustomDisplay<'_, DeviceFormat> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let has_additional_options = !self.0.additional_options.is_nil();
		device_tree_fmt(f, &self.0.device, 0, has_additional_options)?;

		if has_additional_options {
			write!(f, "\n╰─ Options: {}", self.0.additional_options.braced())?;
		}

		Ok(())
	}
}

fn device_tree_fmt(
	f: &mut std::fmt::Formatter<'_>,
	device: &Device,
	level: usize,
	has_extra: bool,
) -> std::fmt::Result {
	let terminal_sign = if has_extra { "├─" } else { "╰─" };
	match device {
		Device::Partition(partition) => {
			match partition {
				Partition::Mbr {
					partition, disk, ..
				} => write!(
					f,
					"Partition (MBR)\n{indent}├─ Disk: {disk:x?}\n{indent}╰─ Partition: {partition:x?}",
					indent = Indent(level),
				),
				Partition::Gpt {
					partition, disk, ..
				} => write!(
					f,
					"Partition (GPT)\n{indent}├─ Disk: {}\n{indent}╰─ Partition: {}",
					disk.braced(),
					partition.braced(),
					indent = Indent(level),
				),
				_ => f.write_str("<unknown partition>"),
			}?;
		}
		Device::File(file) => {
			write!(
				f,
				"File\n{indent}├─ Path: {}\n{indent}{terminal_sign} In: ",
				file.path,
				indent = Indent(level)
			)?;
			device_tree_fmt(f, &file.device, level + 1, false)?;
		}
		Device::Ramdisk(_ramdisk) => f.write_str("Ramdisk <?>")?,
		_ => f.write_str("<unknown>")?,
	}

	Ok(())
}

struct Indent(usize);
impl Display for Indent {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		std::iter::repeat_n("   ", self.0).try_for_each(|string| f.write_str(string))
	}
}
