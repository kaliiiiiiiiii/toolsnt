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

impl TryFrom<u32> for ObjectType {
	type Error = ();

	fn try_from(tag: u32) -> Result<Self, Self::Error> {
		let type_ = ((tag & 0xF0000000) >> 28) as u8;
		let subtype = ((tag & 0x00F00000) >> 20) as u8;
		let app_type = (tag & 0x000FFFFF) as u8;

		let result = match type_ {
			1 => Self::Application {
				image: ImageType::try_from(subtype).map_err(|_| ())?,
				app_type: ApplicationType::from(app_type),
			},
			2 => Self::Inherit(InheritType::try_from(subtype).map_err(|_| ())?),
			3 => Self::Device,
			_ => return Err(()),
		};

		Ok(result)
	}
}

impl From<ObjectType> for u32 {
	fn from(value: ObjectType) -> Self {
		let (type_, subtype, app_type);
		match value {
			ObjectType::Application {
				image,
				app_type: app_type_tag,
			} => {
				type_ = 1;
				subtype = u8::from(image);
				app_type = u8::from(app_type_tag);
			}
			ObjectType::Inherit(inherit_type) => {
				type_ = 2;
				subtype = u8::from(inherit_type);
				app_type = 0;
			}
			ObjectType::Device => {
				type_ = 3;
				subtype = 0;
				app_type = 0;
			}
		}

		((type_ as u32) << 28) | ((subtype as u32) << 20) | (app_type as u32)
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
