//! Here lies all supported BCD elements
//!
//! Elements and their values generated from `elements/elements.kdl` file

use {
	crate::value::Type,
	derive_more::Debug,
	num_enum::{IntoPrimitive, TryFromPrimitive},
	std::{fmt::Display, str::FromStr},
};

/// Class to which the element belongs
#[derive(Clone, Copy, Debug, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum Class {
	/// It is for all elements
	Library = 1,
	/// Only for application elements
	Application,
	/// Only for device elements
	Device,
	/// Only used in BCD templates
	Template,
}

/// Runtime-typechecked element
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[debug("DynamicElement({_0:x})")]
pub struct DynamicElement(u32);
impl DynamicElement {
	/// Create a new [`DynamicElement]` out of its ID
	pub const fn new(val: u32) -> Self {
		Self(val)
	}

	/// Get [`DynamicElement`]'s ID
	pub const fn as_raw(self) -> u32 {
		self.0
	}

	/// Get it's name (if known)
	pub const fn name(&self) -> Option<&'static str> {
		__element_to_str(self.as_raw())
	}
}

impl Display for DynamicElement {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		if let Some(string) = self.name() {
			f.write_str(string)
		} else {
			write!(f, "Unknown ({:x})", self.as_raw())
		}
	}
}

impl FromStr for DynamicElement {
	type Err = FromStrError;

	fn from_str(s: &str) -> Result<Self, Self::Err> {
		__element_from_str(s).ok_or(FromStrError)
	}
}

/// Element doesn't have a corresponding string
#[derive(Clone, Copy, Debug, PartialEq, Eq, derive_more::Display, derive_more::Error)]
#[display("Could not find an element corresponding to specified string")]
pub struct FromStrError;

/// Get a type of object ID
pub fn value_type_of_id(id: u32) -> Option<Type> {
	let typetag = (id >> 24) & 0xf;
	Type::try_from_primitive(typetag as u8).ok()
}

impl Enum {
	/// Choose [`Enum`] variant for the element ID and convert the value
	pub fn from_dword_for_element(id: u32, value: u64) -> Option<Self> {
		Self::__for_element_value(id, value)
	}
}

bcdedit_macros::define_elements!();
pub use generated::*;
