#![forbid(unsafe_op_in_unsafe_fn)]

pub mod elements;
pub mod object;
pub mod value;
pub mod well_known;

use {
	derive_more::{Display, Error},
	error_stack::{bail, report, Result, ResultExt},
	hivex::{node::NodeHandle, CommitFlags, Hive, LibCBox, OpenFlags, SetValueFlags},
	object::{typing::ObjectType, Object},
	smallstr::SmallString,
	std::{borrow::Cow, ffi::CStr, path::Path},
	uuid::Uuid,
};

// Boot Configuration Data Hive
pub struct Bcd {
	hive: Hive,
	objects_node: NodeHandle,
	description_node: NodeHandle,
}

impl Bcd {
	/// Create [`Bcd`] from opened BCD registry hive
	///
	/// Note that it has to be a valid BCD hive. Opening other hives is not
	/// supported.
	pub fn from_hive(hive: Hive) -> Option<Self> {
		let root = hive.root().ok()?;
		let objects_node = hive.node(root).get_child("Objects")?;
		let description_node = hive.node(root).get_child("Description")?;

		Some(Self {
			hive,
			objects_node,
			description_node,
		})
	}

	/// Create a completely new [`Bcd`]
	pub fn create(
		path: impl AsRef<Path>,
		store_flags: StoreFlags,
		hive_flags: OpenFlags,
	) -> Result<Self, StoreCreationError> {
		let hive = Hive::create(path.as_ref(), hive_flags | OpenFlags::WRITE)
			.change_context(StoreCreationError)?;
		
		let root = hive.node(hive.root().change_context(StoreCreationError)?);

		let description_node = root
			.node_add_child(c"Description")
			.change_context(StoreCreationError)?;

		let objects_node = root
			.node_add_child(c"Objects")
			.change_context(StoreCreationError)?;

		let new = Self {
			hive,
			objects_node,
			description_node,
		};

		new.set_flags(store_flags)
			.change_context(StoreCreationError)?;

		Ok(new)
	}

	/// Commit changes to backing hive file
	pub fn commit(&self) -> Result<(), HivexError> {
		self.hive
			.commit(None, CommitFlags::empty())
			.change_context(HivexError)?;

		Ok(())
	}

	/// Get BCD store flags
	pub fn flags(&self) -> StoreFlags {
		let description = self.hive.node(self.description_node);
		let flag_mask = |id, flag| {
			description
				.get_value(id)
				.map(|value_h| self.hive.value(value_h).downcast_dword() == 1)
				.unwrap_or_default()
				.then_some(flag)
				.unwrap_or(StoreFlags::empty())
		};

		StoreFlags::empty()
			| flag_mask(c"System", StoreFlags::SYSTEM)
			| flag_mask(c"TreatAsSystem", StoreFlags::TREAT_AS_SYSTEM)
	}

	/// Set BCD store flag
	pub fn set_flags(&self, flags: StoreFlags) -> Result<(), std::io::Error> {
		let node = self.hive.node(self.description_node);
		let set_by = |key, flag| {
			let val = flags.contains(flag).then_some(1).unwrap_or_default();
			node.set_value(
				SetValueFlags::empty(),
				key,
				hivex::value::Value::<Cow<CStr>>::Dword(val),
			)
		};

		set_by(c"System", StoreFlags::SYSTEM)?;
		set_by(c"TreatAsSystem", StoreFlags::TREAT_AS_SYSTEM)?;

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

	/// Lookup and select object by UUID
	pub fn object_lookup(&self, uuid: Uuid) -> Result<Option<Object>, ObjectRetrievalError> {
		for handle in self.objects().to_vec() {
			let Ok(name) = self.hive.node(handle.0).name() else {
				continue;
			};

			if Uuid::parse_str(&name).change_context(ObjectRetrievalError::Uuid)? == uuid {
				return self.object(handle).map(Some);
			}
		}

		Ok(None)
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
		let type_tag = object::typing::ObjectType::try_from(type_tag_num)
			.map_err(|_| ObjectRetrievalError::InvalidType)?;

		let uuid = self
			.object_uuid(handle)
			.change_context(ObjectRetrievalError::Uuid)?;

		Ok(Object {
			elements: self.hive.node(elements_node),
			uuid,
			type_tag,
		})
	}

	/// Create a new object
	pub fn object_create(
		&self,
		uuid: MaybeUuid,
		type_: ObjectType,
	) -> Result<Object, ObjectCreationError> {
		let uuid = match uuid {
			MaybeUuid::Generate => Uuid::new_v4(),
			MaybeUuid::Provided(uuid) => uuid,
		};

		let type_tag = u32::from(type_);
		let objects_node = self.hive.node(self.objects_node);

		let node = {
			let mut uuid_buf = [0_u8; 128];
			let uuid = uuid.braced().encode_lower(&mut uuid_buf);
			let handle = match objects_node.node_add_child(&*uuid) {
				Ok(h) => h,
				Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
					bail!(ObjectCreationError::AlreadyExists)
				}
				Err(e) => bail!(report!(e).change_context(ObjectCreationError::Hivex)),
			};

			self.hive.node(handle)
		};

		let description = self.hive.node(
			node.node_add_child(c"Description")
				.change_context(ObjectCreationError::Hivex)?,
		);

		description
			.set_value(
				SetValueFlags::empty(),
				c"Type",
				hivex::value::Value::<Cow<CStr>>::Dword(type_tag),
			)
			.change_context(ObjectCreationError::Hivex)?;

		let elements = self.hive.node(
			node.node_add_child(c"Elements")
				.change_context(ObjectCreationError::Hivex)?,
		);

		Ok(Object {
			elements,
			uuid,
			type_tag: type_,
		})
	}
}

#[derive(Clone, Copy)]
pub enum MaybeUuid {
	Generate,
	Provided(Uuid),
}

#[derive(Clone, Copy)]
pub enum OpenMode {
	ReadOnly,
	ReadWrite,
}

bitflags::bitflags! {
	/// BCD Store flags
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub struct StoreFlags: u8 {
		const SYSTEM = 0b1;
		const TREAT_AS_SYSTEM = 0b10;
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectHandle(NodeHandle);

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
#[display("Registry manipulation error")]
pub struct HivexError;

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum UuidRetrievalError {
	#[display("Registry read error")]
	Hivex,
	#[display("UUID parse error")]
	Parse,
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub enum ObjectCreationError {
	#[display("Object with same UUID already exists")]
	AlreadyExists,
	#[display("BCD hive manipulation error")]
	Hivex,
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub struct StoreCreationError;

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
