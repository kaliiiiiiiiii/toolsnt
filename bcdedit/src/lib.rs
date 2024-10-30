#![forbid(unsafe_op_in_unsafe_fn)]

pub mod elements;
pub mod object;
pub mod value;

use {
	derive_more::{Display, Error},
	error_stack::{Result, ResultExt},
	hivex::{node::NodeHandle, CommitFlags, Hive, LibCBox},
	object::Object,
	smallstr::SmallString,
	uuid::Uuid,
};

// Boot Configuration Data Hive
pub struct Bcd {
	hive: Hive,
	objects_node: NodeHandle,
}

impl Bcd {
	/// Create [`Bcd`] from opened BCD registry hive
	///
	/// Note that it has to be a valid BCD hive. Opening other hives is not
	/// supported.
	pub fn from_hive(hive: Hive) -> Option<Self> {
		let root = hive.root().ok()?;
		let objects_node = hive.node(root).get_child("Objects")?;

		Some(Self { hive, objects_node })
	}

	/// Commit changes to backing hive file
	pub fn commit(&self) -> Result<(), HivexError> {
		self.hive
			.commit(None, CommitFlags::empty())
			.change_context(HivexError)?;

		Ok(())
	}

	/// List objects inside BCD
	///
	/// These are not cached so they are being retrieved every time this
	/// function is called.
	pub fn objects(&self) -> LibCBox<[ObjectHandle]> {
		// SAFETY: [`ObjectHandle`] is a repr-transparent wrapper, should be fine
		unsafe { std::mem::transmute(self.hive.node(self.objects_node).children()) }
	}

	/// Get UUID of object
	pub fn object_uuid(&self, handle: ObjectHandle) -> Result<Uuid, UuidRetrievalError> {
		let name = self
			.hive
			.node(handle.0)
			.name()
			.change_context(UuidRetrievalError::Hivex)?;

		Uuid::parse_str(&name).change_context(UuidRetrievalError::Parse)
	}

	/// Retrieve object from handle
	pub fn object(&self, handle: ObjectHandle) -> Result<Object, ObjectRetrievalError> {
		let node = self.hive.node(handle.0);

		// Find the object's node children
		let (description_node, elements_node);
		{
			// We may or may not find them. Look through all children and try.
			let (mut maybe_description_node, mut maybe_elements_node) = (None, None);
			for child in node.children().into_vec() {
				let name = self
					.hive
					.node(child)
					.name()
					.expect("The node was in child list, it should have a name");

				// May be nul-terminated.
				match name.trim_matches('\0') {
					"Description" => maybe_description_node = Some(child),
					"Elements" => maybe_elements_node = Some(child),
					_ => (),
				}
			}

			// We need to have them
			description_node = maybe_description_node
				.ok_or(ObjectRetrievalError::MissingTypeInfo)
				.attach_printable(
					"Node lacks a `Description` node. It is not a valid BCD object",
				)?;

			elements_node = maybe_elements_node.ok_or(ObjectRetrievalError::MissingElements)?;
		}

		let type_tag_handle = self
			.hive
			.node(description_node)
			.get_value(c"Type")
			.change_context(ObjectRetrievalError::MissingTypeInfo)
			.attach_printable("Failed to retrieve `Description\\Type` value handle")?;

		let type_tag_num = self.hive.value(type_tag_handle).downcast_dword();
		let type_tag = object::typing::ObjectType::from_tag(type_tag_num)
			.ok_or(ObjectRetrievalError::InvalidType)?;

		let uuid = self
			.object_uuid(handle)
			.change_context(ObjectRetrievalError::Uuid)?;

		Ok(Object {
			elements: self.hive.node(elements_node),
			uuid,
			type_tag,
		})
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectHandle(NodeHandle);

#[derive(Debug, Display, Error)]
#[display("Registry manipulation error")]
pub struct HivexError;

#[derive(Debug, Display, Error)]
pub enum UuidRetrievalError {
	#[display("Registry read error")]
	Hivex,
	#[display("UUID parse error")]
	Parse,
}

#[derive(Debug, Display, Error)]
pub enum ObjectRetrievalError {
	#[display("Missing type information")]
	MissingTypeInfo,
	#[display("Missing elements key")]
	MissingElements,
	#[display("Invalid UUID")]
	Uuid,
	#[display("Type tag in the hive is not valid or a supported value")]
	InvalidType,
}

fn hex_of_u32(num: u32) -> SmallString<[u8; 8]> {
	use std::fmt::Write as _;
	let mut buffer = SmallString::new();
	write!(buffer, "{num:08x}").unwrap();
	buffer
}

#[cfg(test)]
mod tests {
	use {super::*, assert2::check};

	#[test]
	fn u32_hex() {
		let expected = "00000020";
		let converted = hex_of_u32(0x20);
		check!(expected == &converted[..]);
	}
}
