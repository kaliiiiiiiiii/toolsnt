use {
	super::{application::Application, device::Device, inherit::Inherit, Any as ObjectType},
	crate::object::{
		application::{app_type::Any as AppType, ImageType},
		inherit::inherit_for::Any as InheritFor,
	},
};

mycelium_bitfield::bitfield! {
	pub struct TypeTag<u32> {
		pub const OBJECT_TYPE = 4;
			const _PADDING    = 1;
		pub const SUBTYPE     = 4;
		pub const APP_TYPE    = 20;
	}
}

pub fn from_tag(tag_num: u32) -> Option<ObjectType> {
	let tag_bitfields = TypeTag::from_bits(tag_num);

	let object_type = tag_bitfields.get(TypeTag::OBJECT_TYPE);
	match object_type {
		1 => from_tag_application(tag_bitfields),
		2 => from_tag_inherit(tag_bitfields),
		3 => Some(ObjectType::Device(Device)),
		_ => None,
	}
}

fn from_tag_inherit(tag_bitfields: TypeTag) -> Option<ObjectType> {
	let inherit_for_tag = tag_bitfields.get(TypeTag::SUBTYPE);
	let inherit_for = match inherit_for_tag {
		1 => InheritFor::All,
		2 => InheritFor::Application,
		3 => InheritFor::Device,
		_ => return None,
	};

	Some(ObjectType::Inherit(Inherit { inherit_for }))
}

fn from_tag_application(tag_bitfields: TypeTag) -> Option<ObjectType> {
	let image_type_tag = tag_bitfields.get(TypeTag::SUBTYPE);
	let image_type = match image_type_tag {
		1 => ImageType::Firmware,
		2 => ImageType::WindowsBoot,
		3 => ImageType::LegacyLoader,
		4 => ImageType::RealMode,
		_ => return None,
	};

	let application_type_tag = tag_bitfields.get(TypeTag::APP_TYPE);
	let application_type = match application_type_tag {
		1 => AppType::FwBootMgr,
		2 => AppType::BootApp,
		3 => AppType::OsLoader,
		4 => AppType::Resume,
		5 => AppType::MemDiag,
		6 => AppType::NtLdr,
		7 => AppType::SetupLdr,
		8 => AppType::BootSector,
		9 => AppType::Startup,
		10 => AppType::BootApp,
		_ => return None,
	};

	Some(ObjectType::Application(Application {
		application_type,
		image_type,
	}))
}

pub fn into_tag(type_: ObjectType) -> u32 {
	let (object_type, subtype, app_type);
	match type_ {
		ObjectType::Application(application) => {
			object_type = 1;
			subtype = application.image_type as u32 + 1;
			app_type = application.application_type as u32 + 1;
		}
		ObjectType::Inherit(inherit) => {
			object_type = 2;
			subtype = inherit.inherit_for as u32 + 1;
			app_type = 0;
		}
		ObjectType::Device(_) => {
			object_type = 3;
			subtype = 0;
			app_type = 0;
		}
	}

	TypeTag::new()
		.with(TypeTag::OBJECT_TYPE, object_type)
		.with(TypeTag::SUBTYPE, subtype)
		.with(TypeTag::APP_TYPE, app_type)
		.bits()
}
