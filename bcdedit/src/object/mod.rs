pub mod application;
pub mod device;
pub mod inherit;
pub mod typetag;

use {
	self::inherit::{CanInherit, Inherit},
	crate::{
		element::{
			format::{Format, IntoFormatSet},
			Element,
		},
		typesystem::{is_subclass_of_self, DowncastExt, SubclassOf},
		HivexError,
	},
	derive_more::{Display, Error},
	error_stack::{report, Result, ResultExt},
	hivex::{
		node::{NodeHandle, SelectedNode},
		SetValueFlags,
	},
	uuid::Uuid,
};

/// Any type of a BCD [`Object`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Any {
	Application(application::Application<application::app_type::Any>),
	Inherit(inherit::Inherit<inherit::inherit_for::Any>),
	Device(device::Device),
}

is_subclass_of_self!(Any);

/// BCD Object
///
/// The type is later determined
pub struct Object<'hive, Type> {
	pub(crate) type_: Type,
	pub(crate) elements: SelectedNode<'hive>,
	pub(crate) uuid: Uuid,
}

impl<'hive, Type> Object<'hive, Type> {
	/// Get object type
	pub const fn object_type(&self) -> &Type {
		&self.type_
	}

	/// Get object's UUID
	pub const fn uuid(&self) -> &Uuid {
		&self.uuid
	}

	/// Get hive node handle
	pub const fn hive_node(&self) -> NodeHandle {
		self.elements.handle()
	}

	/// Downcast [`Object`]
	pub fn downcast<To>(self) -> std::result::Result<Object<'hive, To>, Self>
	where
		To: SubclassOf<Type>,
	{
		let type_ = match self.type_.downcast() {
			Ok(type_) => type_,
			Err(type_) => {
				return Err(Self {
					type_,
					elements: self.elements,
					uuid: self.uuid,
				})
			}
		};

		Ok(Object {
			type_,
			elements: self.elements,
			uuid: self.uuid,
		})
	}

	/// Upcast [`Object`]
	pub fn upcast<To>(self) -> Object<'hive, To>
	where
		Type: SubclassOf<To>,
	{
		Object {
			type_: self.type_.upcast(),
			elements: self.elements,
			uuid: self.uuid,
		}
	}

	/// Query an element without statically checking if it is compatible with
	/// object type
	pub fn get_unchecked<E>(&self, _key: E) -> Result<<E::Format as Format>::Get, GetError>
	where
		E: Element,
		E::Format: Format,
	{
		let hive = self.elements.hive();
		let node_handle = self
			.elements
			.get_child(E::ID)
			.ok_or_else(|| report!(GetError::ElementNotFound))?;

		let value_handle = hive
			.node(node_handle)
			.get_value("Element")
			.map_err(HivexError)
			.change_context(GetError::ElementMissingValue)?;

		let value = hive
			.value(value_handle)
			.get()
			.map_err(HivexError)
			.change_context(GetError::HiveGetValue)?;

		E::Format::from_hive_value(value).change_context(GetError::ValueConversion)
	}

	/// Set an element without statically checking if it is compatible with
	/// object type
	pub fn set_unchecked<'a, E>(
		&self,
		_key: E,
		value: impl IntoFormatSet<'a, E::Format>,
	) -> Result<(), SetError>
	where
		E: Element,
		E::Format: Format + 'a,
	{
		let hive = self.elements.hive();

		let maybe_node_handle = self.elements.get_child(E::ID);

		let node_handle = match maybe_node_handle {
			Some(handle) => handle,
			None => self
				.elements
				.node_add_child(E::ID)
				.change_context(SetError::ElementKeyCreation)?,
		};

		let format_set = value.into_format_set();
		let hive_value = E::Format::into_hive_value(format_set);

		hive.node(node_handle)
			.set_value(SetValueFlags::empty(), "Element", hive_value)
			.change_context(SetError::Hive)
			.attach_printable("Attempted to set element's value")
	}

	pub fn add_inherit<I>(&self, inherit: &Object<Inherit<I>>) -> Result<(), InheritChangeError>
	where
		Type: CanInherit<I> + SubclassOf<Any>,
	{
		let mut current = self
			.get_unchecked(crate::element::library::Inherit)
			.change_context(InheritChangeError::Get)?
			.into_vec();

		current.push(inherit.uuid);

		self.set_unchecked(crate::element::library::Inherit, &current[..])
			.change_context(InheritChangeError::Set)
	}

	pub fn remove_inherit<I>(&self, inherit: &Object<Inherit<I>>) -> Result<(), InheritChangeError>
	where
		Type: CanInherit<I> + SubclassOf<Any>,
	{
		let mut current = self
			.get_unchecked(crate::element::library::Inherit)
			.change_context(InheritChangeError::Get)?
			.into_vec();

		current.retain(|uuid| *uuid != inherit.uuid);
		let slice = &current[..];

		self.set_unchecked(crate::element::library::Inherit, slice)
			.change_context(InheritChangeError::Set)
	}
}

impl<Type> Object<'_, Type>
where
	Type: IsNotInherit,
{
	/// Get an element
	pub fn get<E>(&self, key: E) -> Result<<E::Format as Format>::Get, GetError>
	where
		E: Element,
		E::Format: Format,
		Type: SubclassOf<E::Class>,
	{
		self.get_unchecked(key)
	}

	/// Set an element
	pub fn set<'a, E>(
		&self,
		key: E,
		value: impl IntoFormatSet<'a, E::Format>,
	) -> Result<(), SetError>
	where
		E: Element,
		E::Format: Format + 'a,
		Type: SubclassOf<E::Class>,
	{
		self.set_unchecked(key, value)
	}
}

impl<For> Object<'_, Inherit<For>> {
	/// Get an element
	pub fn get<E>(&self, key: E) -> Result<<E::Format as Format>::Get, GetError>
	where
		E: Element,
		E::Format: Format,
		E::Class: inherit::CanInherit<For>,
	{
		self.get_unchecked(key)
	}

	/// Set an element
	pub fn set<'a, E>(
		&self,
		key: E,
		value: impl IntoFormatSet<'a, E::Format>,
	) -> Result<(), SetError>
	where
		E: Element,
		E::Format: Format + 'a,
		E::Class: inherit::CanInherit<For>,
	{
		self.set_unchecked(key, value)
	}
}
#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum InheritChangeError {
	Get,
	Set,
}

/// Defines subclassing based of subclassing of inner type
impl<'a, SuperT, SubT> SubclassOf<Object<'a, SuperT>> for Object<'a, SubT>
where
	SubT: SubclassOf<SuperT>,
{
	fn upcast(self) -> Object<'a, SuperT> {
		Object {
			type_: self.type_.upcast(),
			elements: self.elements,
			uuid: self.uuid,
		}
	}

	fn downcast_from(above: Object<'a, SuperT>) -> std::result::Result<Self, Object<'a, SuperT>>
	where
		Self: Sized,
	{
		let node = above.elements;
		match above.type_.downcast() {
			Ok(type_) => Ok(Self {
				type_,
				elements: node,
				uuid: above.uuid,
			}),
			Err(type_) => Err(Object {
				type_,
				elements: node,
				uuid: above.uuid,
			}),
		}
	}
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum GetError {
	#[display = "Element misses value"]
	ElementMissingValue,
	#[display = "Failed to get value of element from hive"]
	HiveGetValue,
	#[display = "Queried element was not found"]
	ElementNotFound,
	#[display = "Value conversion"]
	ValueConversion,
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum SetError {
	#[display = "Hive manipulation error"]
	Hive,
	#[display = "Failed to create element key"]
	ElementKeyCreation,
	#[display = "Value conversion"]
	ValueConversion,
}

pub trait IsNotInherit {}
impl<Any> IsNotInherit for application::Application<Any> {}
impl IsNotInherit for device::Device {}
