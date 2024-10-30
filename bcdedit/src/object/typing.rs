use num_enum::{FromPrimitive, IntoPrimitive, TryFromPrimitive};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObjectType {
	Application {
		image: ImageType,
		app_type: ApplicationType,
	},
	Inherit(InheritType),
	Device,
}

impl ObjectType {
	pub fn from_tag(tag: u32) -> Option<Self> {
		let type_ = ((tag & 0xF0000000) >> 28) as u8;
		let subtype = ((tag & 0x00F00000) >> 20) as u8;
		let app_type = (tag & 0x000FFFFF) as u8;

		let result = match type_ {
			1 => Self::Application {
				image: ImageType::try_from(subtype).ok()?,
				app_type: ApplicationType::from(app_type),
			},
			2 => Self::Inherit(InheritType::try_from(subtype).ok()?),
			3 => Self::Device,
			_ => return None,
		};

		Some(result)
	}
}

/// Type of image for load
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ImageType {
	Firmware = 1,
	WindowsBoot,
	LegacyLoader,
	RealMode,
}

/// Type of application
#[derive(Clone, Copy, Debug, PartialEq, Eq, FromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ApplicationType {
	FwBootMgr = 1,
	BootMgr,
	OsLoader,
	Resume,
	MemDiag,
	NtLdr,
	SetupLdr,
	BootSector,
	Startup,
	BootApp,
	#[num_enum(default)]
	Unknown = 255,
}

/// Inheritable by what
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum InheritType {
	/// By any object
	Any = 1,
	/// By [`ObjectType::Application`]
	Application,
	/// By [`ObjectType::Device`]
	Device,
}
