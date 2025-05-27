//! Library for reading and editing Windows Boot Configuration Data

#![deny(missing_docs)]
#![forbid(unsafe_op_in_unsafe_fn)]

pub mod elements;
pub mod object;
pub mod value;
pub mod well_known;

pub use hivex::OpenFlags;

use {
	derive_more::{Display, Error},
	hivex::{node::NodeHandle, CommitFlags, Hive, LibCBox, SetValueFlags},
	object::{typing::ObjectType, Object},
	smallstr::SmallString,
	std::{borrow::Cow, ffi::CStr, io::Result as IoResult, path::Path},
	uuid::Uuid,
};

/// Boot Configuration Data store
pub struct Store {
	hive: Hive,
	objects_node: NodeHandle,
	description_node: NodeHandle,
}

impl Store {
	/// Create [`Store`] from opened BCD registry hive
	///
	/// Note that it has to be a valid BCD hive. Opening other hives is not
	/// supported.
	pub fn from_hive(hive: Hive) -> IoResult<Self> {
		let io_error = |msg| std::io::Error::new(std::io::ErrorKind::InvalidInput, msg);

		let root = hive.root()?;
		let objects_node = hive
			.node(root)
			.get_child("Objects")
			.ok_or_else(|| io_error("\"Objects\" node was not found"))?;

		let description_node = hive
			.node(root)
			.get_child("Description")
			.ok_or_else(|| io_error("\"Description\" node was not found"))?;

		Ok(Self {
			hive,
			objects_node,
			description_node,
		})
	}

	/// Open [`Store`] by it's path and [`OpenFlags`].
	///
	/// Calls internally [`Hive::open`] and [`Store::from_hive`]
	pub fn open(path: impl AsRef<Path>, flags: OpenFlags) -> IoResult<Self> {
		let hive = Hive::open(path, flags)?;
		Self::from_hive(hive)
	}

	/// Create a completely new [`Store`]
	pub fn create(
		path: impl AsRef<Path>,
		store_flags: StoreFlags,
		hive_flags: OpenFlags,
	) -> IoResult<Self> {
		let hive = Hive::create(path.as_ref(), hive_flags | OpenFlags::WRITE)?;
		let root = hive.node(hive.root()?);

		let description_node = root.node_add_child(c"Description")?;
		let objects_node = root.node_add_child(c"Objects")?;
		let new = Self {
			hive,
			objects_node,
			description_node,
		};

		new.set_flags(store_flags)?;

		Ok(new)
	}

	/// Delete object from [`Store`]
	pub fn delete_object(&self, handle: ObjectHandle) -> IoResult<()> {
		self.hive.node(handle.0).delete()
	}

	/// Commit changes to backing hive file
	pub fn commit(&self) -> IoResult<()> {
		self.hive.commit(None, CommitFlags::empty())
	}

	/// Get BCD store flags
	pub fn flags(&self) -> StoreFlags {
		let description = self.hive.node(self.description_node);
		let flag_mask = |id, flag| {
			if description
				.get_value(id)
				.map(|value_h| self.hive.value(value_h).downcast_dword() == 1)
				.unwrap_or_default()
			{
				flag
			} else {
				StoreFlags::empty()
			}
		};

		StoreFlags::empty()
			| flag_mask(c"System", StoreFlags::SYSTEM)
			| flag_mask(c"TreatAsSystem", StoreFlags::TREAT_AS_SYSTEM)
	}

	/// Set BCD store flag
	pub fn set_flags(&self, flags: StoreFlags) -> IoResult<()> {
		let node = self.hive.node(self.description_node);
		let set_by = |key, flag| {
			let val = flags.contains(flag) as u32;

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
	pub fn object_uuid(&self, handle: ObjectHandle) -> IoResult<Uuid> {
		let name = self.hive.node(handle.0).name()?;
		Uuid::parse_str(&name).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
	}

	/// Lookup and select object by UUID
	pub fn object_lookup(&self, uuid: Uuid) -> Option<Result<Object, FromHiveRetrievalError>> {
		for handle in self.objects().iter().copied() {
			let Ok(obj_uuid) = self.object_uuid(handle) else {
				continue;
			};

			if obj_uuid == uuid {
				return Some(self.object(handle));
			}
		}

		None
	}

	/// Retrieve object from handle
	pub fn object(&self, handle: ObjectHandle) -> Result<Object, FromHiveRetrievalError> {
		(|| {
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
				description_node =
					maybe_description_node.ok_or(FromHiveRetrievalErrorInner::MissingTypeInfo)?;
				elements_node =
					maybe_elements_node.ok_or(FromHiveRetrievalErrorInner::MissingElements)?;
			}

			let type_tag_handle = self
				.hive
				.node(description_node)
				.get_value(c"Type")
				.map_err(|_| FromHiveRetrievalErrorInner::MissingTypeInfo)?;

			let type_tag_num = self.hive.value(type_tag_handle).downcast_dword();
			let type_tag = object::typing::ObjectType::try_from(type_tag_num)
				.map_err(|_| FromHiveRetrievalErrorInner::InvalidType)?;

			let uuid = self
				.object_uuid(handle)
				.map_err(FromHiveRetrievalErrorInner::UuidRetrieval)?;

			Ok(Object {
				handle,
				elements: self.hive.node(elements_node),
				uuid,
				type_tag,
			})
		})()
		.map_err(FromHiveRetrievalError)
	}

	/// Create a new object
	pub fn object_create(&self, uuid: Uuid, type_: ObjectType) -> IoResult<Object> {
		let type_tag = u32::from(type_);
		let objects_node = self.hive.node(self.objects_node);

		let (handle, node);
		{
			let mut uuid_buf = [0_u8; 128];
			let uuid = uuid.braced().encode_lower(&mut uuid_buf);
			handle = objects_node.node_add_child(&*uuid)?;
			node = self.hive.node(handle);
		};

		let description = self.hive.node(node.node_add_child(c"Description")?);
		description.set_value(
			SetValueFlags::empty(),
			c"Type",
			hivex::value::Value::<Cow<CStr>>::Dword(type_tag),
		)?;

		let elements = self.hive.node(node.node_add_child(c"Elements")?);

		Ok(Object {
			handle: ObjectHandle(handle),
			elements,
			uuid,
			type_tag: type_,
		})
	}
}

bitflags::bitflags! {
	/// BCD Store flags
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub struct StoreFlags: u8 {
		/// Tell Windows to mount this store to HKEY_LOCAL_MACHINE (?)
		const SYSTEM = 0b1;
		/// Treat as system store
		const TREAT_AS_SYSTEM = 0b10;
	}
}

/// Handle to BCD object
///
/// Wraps [`NodeHandle`]
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectHandle(NodeHandle);

/// Error context for retreiving object from the store
#[derive(Debug, Display, Error)]
pub struct FromHiveRetrievalError(FromHiveRetrievalErrorInner);

/// Inner represenation of object retrieval failure
#[derive(Debug, Display, Error)]
enum FromHiveRetrievalErrorInner {
	/// There was no type information
	#[display("Missing type information")]
	MissingTypeInfo,
	/// There are no elements (Description\Type)
	#[display("Missing elements key")]
	MissingElements,
	/// The type of the object is not valid or BCDEdit doesn't support it
	/// (hopefully yet)
	#[display("Type tag in the hive is not valid or a supported value")]
	InvalidType,
	/// UUID retrieval error
	#[display("Object UUID is not correct")]
	UuidRetrieval(std::io::Error),
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
