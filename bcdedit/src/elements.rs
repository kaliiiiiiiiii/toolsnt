use {
	crate::value::Type,
	derive_more::Debug,
	num_enum::{IntoPrimitive, TryFromPrimitive},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Class {
	Library = 1,
	Application,
	Device,
	Template,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[debug("DynamicElement({_0:x})")]
pub struct DynamicElement(u32);
impl DynamicElement {
	pub const fn new(val: u32) -> Self {
		Self(val)
	}

	pub const fn as_raw(self) -> u32 {
		self.0
	}
}

pub fn value_type_of_id(id: u32) -> Option<Type> {
	let typetag = (id >> 24) & 0xf;
	Type::try_from_primitive(typetag as u8).ok()
}

bcdedit_macros::define_elements!();
