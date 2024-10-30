use {
	super::{
		device::DeviceFormat,
		format::{self, Format},
	},
	num_enum::{IntoPrimitive, TryFromPrimitive},
	std::fmt::Debug,
};

pub type GetValue<'a> = Value<'a, Get>;
pub type SetValue<'a> = Value<'a, Set>;

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Value<'a, Sel: UseCase> {
	Device(Sel::Value<'a, DeviceFormat>) = 1,
	String(Sel::Value<'a, format::Str>),
	Guid(Sel::Value<'a, format::Guid>),
	GuidList(Sel::Value<'a, format::GuidList>),
	Integer(Sel::Value<'a, format::Integer>),
	Bool(Sel::Value<'a, format::Bool>),
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum Type {
	Device = 1,
	String,
	Guid,
	GuidList,
	Integer,
	Bool,
	IntegerList,
}

pub trait UseCase {
	type Value<'a, T: Format>;
}

pub enum Set {}
impl UseCase for Set {
	type Value<'a, T: Format> = T::Set<'a>;
}

pub enum Get {}
impl UseCase for Get {
	type Value<'a, T: Format> = T::Get;
}
