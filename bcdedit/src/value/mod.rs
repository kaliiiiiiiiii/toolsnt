//! Rust representations of BCD's values
//!
//! # Rant
//! BCD is both overengineered and underengineered. Windows Registry has funny
//! things, in my homeland we call them types.
//!
//! It does even have `REG_DWORD` type, an [integer][`format::Integer`] type.
//! But no, Microsoft has to pretend it doesn't exist and encode a DWORD
//! ([`u64`]) as `REG_BINARY` (slice of bytes).
//!
//! And you wouldn't guess how they encode [UUIDs][`format::Guid`]. You have
//! here your beloved `REG_BINARY`, Microsoft. It could have been 16 bytes. But
//! no! You had to choose UTF-16 pretty representation of UUID. The braced
//! format with the dashes. 39 bytes instead of 16.
//!
//! And don't get me started on the [Device][`device`] format.
//!
//! Congratulations, Microsoft. Good job. /s

pub mod device;
pub mod format;

mod value;

use {
	device::DeviceFormat,
	format::Format,
	num_enum::{IntoPrimitive, TryFromPrimitive},
	std::fmt::Debug,
};

/// Value when getting from the store
pub type GetValue<'a> = Value<'a, Get>;
/// Value when saving to the store
pub type SetValue<'a> = Value<'a, Set>;

/// Enum of all supported value formats
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Value<'a, Sel: UseCase> {
	/// A device
	Device(Sel::Value<'a, DeviceFormat>) = 1,
	/// A string
	String(Sel::Value<'a, format::Str>),
	/// A GUID (UUID)
	Guid(Sel::Value<'a, format::Guid>),
	/// A list of GUIDs
	GuidList(Sel::Value<'a, format::GuidList>),
	/// An integer
	Integer(Sel::Value<'a, format::Integer>),
	/// A boolean
	Bool(Sel::Value<'a, format::Bool>),
	/// A list of integers
	IntegerList(Sel::Value<'a, format::IntegerList>),
}

impl<'a, Sel: UseCase> Debug for Value<'a, Sel>
where
	Sel::Value<'a, DeviceFormat>: Debug,
	Sel::Value<'a, format::Str>: Debug,
	Sel::Value<'a, format::Guid>: Debug,
	Sel::Value<'a, format::GuidList>: Debug,
	Sel::Value<'a, format::Integer>: Debug,
	Sel::Value<'a, format::Bool>: Debug,
	Sel::Value<'a, format::IntegerList>: Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			Self::Device(arg0) => f.debug_tuple("Device").field(arg0).finish(),
			Self::String(arg0) => f.debug_tuple("String").field(arg0).finish(),
			Self::Guid(arg0) => f.debug_tuple("Guid").field(arg0).finish(),
			Self::GuidList(arg0) => f.debug_tuple("GuidList").field(arg0).finish(),
			Self::Integer(arg0) => f.debug_tuple("Integer").field(arg0).finish(),
			Self::Bool(arg0) => f.debug_tuple("Bool").field(arg0).finish(),
			Self::IntegerList(arg0) => f.debug_tuple("IntegerList").field(arg0).finish(),
		}
	}
}

impl<Sel: UseCase> Value<'_, Sel> {
	/// Get type of the value
	pub fn type_of(&self) -> Type {
		match self {
			Value::Device(_) => Type::Device,
			Value::String(_) => Type::String,
			Value::Guid(_) => Type::Guid,
			Value::GuidList(_) => Type::GuidList,
			Value::Integer(_) => Type::Integer,
			Value::Bool(_) => Type::Bool,
			Value::IntegerList(_) => Type::IntegerList,
		}
	}
}

/// Type of a value
#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Type {
	/// A device
	Device = 1,
	/// A string
	String,
	/// A GUID (UUID)
	Guid,
	/// A list of GUIDs
	GuidList,
	/// An integer
	Integer,
	/// A boolean
	Bool,
	/// A list of integers
	IntegerList,
}

/// Specifies use case of type marker
pub trait UseCase {
	/// Value selector
	type Value<'a, T: Format>;
}

/// Marker of value to be stored
pub enum Set {}
impl UseCase for Set {
	type Value<'a, T: Format> = T::Set<'a>;
}

/// Marker of value to be retrieved
pub enum Get {}
impl UseCase for Get {
	type Value<'a, T: Format> = T::Get;
}
