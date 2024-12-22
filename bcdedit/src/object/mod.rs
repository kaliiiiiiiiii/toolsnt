//! BCD Object

pub mod elements_iter;
pub mod typing;

use {
	crate::{
		elements::{value_type_of_id, DynamicElement},
		hex_of_u32,
		value::{
			format::{self, Format},
			GetValue, SetValue, Type, Value,
		},
		ObjectDeletionError, ObjectHandle,
	},
	derive_more::{Display, Error},
	error_stack::{ensure, report, Result, ResultExt},
	hivex::{
		node::{NodeHandle, SelectedNode},
		BorrowedHive, SetValueFlags,
	},
	std::fmt::Debug,
	typing::ObjectType,
	uuid::Uuid,
};

/// BCD Object
pub struct Object<'hive> {
	pub(crate) handle: ObjectHandle,
	pub(crate) elements: SelectedNode<'hive>,
	pub(crate) uuid: Uuid,
	pub(crate) type_tag: ObjectType,
}

impl Debug for Object<'_> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("Object")
			.field("uuid", &self.uuid)
			.field("type_tag", &self.type_tag)
			.finish_non_exhaustive()
	}
}

impl<'hive> Object<'hive> {
	/// Get object's UUID
	pub const fn uuid(&self) -> &Uuid {
		&self.uuid
	}

	/// Get object's type
	pub const fn type_(&self) -> ObjectType {
		self.type_tag
	}

	/// Delete self from the BCD store
	pub fn delete(self) -> Result<(), ObjectDeletionError> {
		self.elements
			.hive()
			.node(self.handle.0)
			.delete()
			.change_context(ObjectDeletionError)
	}

	/// Set a [`Value`] in BCD
	///
	/// Do not forget to commit to the BCD store to save the changes by
	/// [`crate::Store::commit`].
	///
	/// # Warning
	/// Object type checking is not yet done. Setting element not belonging to
	/// format of this object may lead to unexpected behaviour.
	pub fn set(&self, element: DynamicElement, value: SetValue) -> Result<(), SetError> {
		macro_rules! unwrap_value {
			($expr:expr, $variant:ident) => {
				if let Value::$variant(x) = $expr {
					x
				} else {
					unreachable!()
				}
			};
		}

		let id = element.as_raw();
		let type_ = value_type_of_id(id)
			.ok_or_else(|| report!(SetError::TypeMismatch))
			.attach_printable(
				"Failed to determine type from the ID. It is contained in 2nd doublet.",
			)?;

		ensure!(type_ == value.type_of(), SetError::TypeMismatch);

		let key = hex_of_u32(id);
		let node_h = match self.elements.get_child(&key) {
			Some(h) => h,
			None => self
				.elements
				.node_add_child(&key[..])
				.change_context(SetError::ElementCreation)?,
		};

		// todo: eww
		let value = match type_ {
			Type::Device => format::DeviceFormat::into_hive_value(unwrap_value!(value, Device)),
			Type::String => format::Str::into_hive_value(unwrap_value!(value, String)),
			Type::Guid => format::Guid::into_hive_value(unwrap_value!(value, Guid)),
			Type::GuidList => format::GuidList::into_hive_value(unwrap_value!(value, GuidList)),
			Type::Integer => format::Integer::into_hive_value(unwrap_value!(value, Integer)),
			Type::Bool => format::Bool::into_hive_value(unwrap_value!(value, Bool)),
			Type::IntegerList => {
				format::IntegerList::into_hive_value(unwrap_value!(value, IntegerList))
			}
		};

		self.elements
			.hive()
			.node(node_h)
			.set_value(SetValueFlags::empty(), "Element", value)
			.change_context(SetError::Set)
	}

	/// Get a [`Value`] element
	pub fn get(&self, element: DynamicElement) -> Result<Option<GetValue>, RetrievalError> {
		let id = element.as_raw();
		let key = hex_of_u32(id);
		let Some(node_h) = self.elements.get_child(&key) else {
			return Ok(None);
		};

		node_get_value(element, self.elements.hive(), node_h)
	}

	/// Iterate through all elements
	pub fn elements(&self) -> elements_iter::Elements<'hive> {
		let handles = self.elements.children().into_vec().into_iter();
		elements_iter::Elements {
			hive: self.elements.hive(),
			handles,
		}
	}
}

fn node_get_value(
	element: DynamicElement,
	hive: BorrowedHive<'_>,
	node_h: NodeHandle,
) -> Result<Option<GetValue>, RetrievalError> {
	let value_h = hive
		.node(node_h)
		.get_value(c"Element")
		.change_context(RetrievalError::MalformedElementInHive)?;

	let value = hive
		.value(value_h)
		.get()
		.change_context(RetrievalError::ValueRetrieval)?;

	let type_ = value_type_of_id(element.as_raw())
		.ok_or_else(|| report!(RetrievalError::UnknownId))
		.attach_printable(
			"Failed to determine type from the ID. Hint: It is contained in 6th doublet.",
		)?;

	match type_ {
		Type::Device => format::DeviceFormat::from_hive_value(value).map(Value::Device),
		Type::String => format::Str::from_hive_value(value).map(Value::String),
		Type::Guid => format::Guid::from_hive_value(value).map(Value::Guid),
		Type::GuidList => format::GuidList::from_hive_value(value).map(Value::GuidList),
		Type::Integer => format::Integer::from_hive_value(value).map(Value::Integer),
		Type::Bool => format::Bool::from_hive_value(value).map(Value::Bool),
		Type::IntegerList => format::IntegerList::from_hive_value(value).map(Value::IntegerList),
	}
	.map(Some)
	.change_context(RetrievalError::FromHiveValue)
}

/// Error context when trying to retrieve an object
#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum RetrievalError {
	/// Missing an `Element` key
	#[display("BCD hive doesn't contain the `Element` key")]
	MalformedElementInHive,
	/// Failed to get it from the hive
	#[display("Failed to get value from the hive")]
	ValueRetrieval,
	/// The ID is not known
	#[display("Provided element ID is not known to BCDEdit")]
	UnknownId,
	/// Value is not correctly formatted or known to BCDEdit
	#[display("Value inside hive is not correctly formatted or known to BCDEdit")]
	FromHiveValue,
}

/// Error context when setting the value
#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum SetError {
	/// Types do not match
	#[display("Type of value doesn't match element's type")]
	TypeMismatch,
	/// Failed to create the element
	#[display("Failed to create element inside hive")]
	ElementCreation,
	/// The ID is not known
	#[display("Provided element ID is not known to BCDEdit")]
	UnknownId,
	/// General hive manipulation error
	#[display("Failed to set value in BCD hive")]
	Set,
}
