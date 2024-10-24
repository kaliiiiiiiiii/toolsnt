#![forbid(unsafe_op_in_unsafe_fn, clippy::missing_const_for_fn)]

pub mod element;
pub mod object;

mod typesystem;

use {
	derive_more::{Display, Error},
	error_stack::{report, Result, ResultExt},
	hivex::{node::NodeHandle, CommitFlags, Hive, LibCBox, SetValueFlags},
	object::Object,
	std::ffi::CString,
	uuid::Uuid,
};

/// Boot Configuration Data
pub struct Bcd {
	hive: Hive,
	objects_node: NodeHandle,
}

impl Bcd {
	pub fn from_hive(hive: Hive) -> Option<Self> {
		let root = hive.root().ok()?;
		let objects = hive.node(root).get_child("Objects")?;

		Some(Self {
			hive,
			objects_node: objects,
		})
	}

	/// Commit changes to backing hive file
	pub fn commit(&self) -> Result<(), HivexError> {
		self.hive
			.commit(None, CommitFlags::empty())
			.map_err(HivexError)?;

		Ok(())
	}

	pub fn objects(&self) -> LibCBox<[ObjectHandle]> {
		// SAFETY: ObjectHandle is a repr-transparent wrapper, should be fine
		unsafe { std::mem::transmute(self.hive.node(self.objects_node).children()) }
	}

	/// Get UUID of object
	pub fn object_uuid(&self, handle: ObjectHandle) -> Result<Uuid, UuidRetrievalError> {
		let name = self
			.hive
			.node(handle.0)
			.name()
			.map_err(HivexError)
			.change_context(UuidRetrievalError::HiveRetrieval)?;

		Uuid::parse_str(&name).change_context(UuidRetrievalError::Uuid)
	}

	/// Get an actual object from an handle
	pub fn object(
		&self,
		handle: ObjectHandle,
	) -> Result<Object<'_, object::Any>, ObjectRetrievalError> {
		let object = self.hive.node(handle.0);

		let (description_handle, elements_handle);
		{
			let (mut maybe_description_handle, mut maybe_elements_handle) = (None, None);
			let children = object.children();
			for child in children.iter().copied() {
				let name = self
					.hive
					.node(child)
					.name()
					.expect("The node was in child list, the name should exist");

				match name.trim_end_matches('\0') {
					"Description" => maybe_description_handle = Some(child),
					"Elements" => maybe_elements_handle = Some(child),
					_ => (),
				}
			}

			description_handle = maybe_description_handle
				.ok_or(ObjectRetrievalError::MissingType)
				.attach_printable("Failed to retreive `Description` node handle")?;

			elements_handle = maybe_elements_handle.ok_or(ObjectRetrievalError::MissingElements)?;
		}
		
		let type_value = self
			.hive
			.node(description_handle)
			.get_value("Type")
			.change_context(ObjectRetrievalError::MissingType)
			.attach_printable("Failed to retrieve `Description\\Type` value handle")?;

		let type_num = self.hive.value(type_value).downcast_dword();
		let type_ = object::typetag::from_tag(type_num).ok_or(ObjectRetrievalError::InvalidType)?;

		let elements = self.hive.node(elements_handle);
		let uuid = self
			.object_uuid(handle)
			.change_context(ObjectRetrievalError::Uuid)?;

		Ok(Object {
			type_,
			elements,
			uuid,
		})
	}

	/// Create a new object
	pub fn new_object<T>(&self, type_: T, uuid: Uuid) -> Result<Object<'_, T>, HivexError>
	where
		T: Copy + typesystem::SubclassOf<object::Any>,
	{
		// Create object
		let Ok(object_node_name) = CString::new(uuid.braced().to_string()) else {
			unreachable!("Formatted UUID (braced) should not contain NUL");
		};

		let object_handle = self
			.hive
			.node(self.objects_node)
			.node_add_child(object_node_name)
			.map_err(HivexError)?;

		let object = self.hive.node(object_handle);

		// Write type
		let description_handle = object.node_add_child(c"Description").map_err(HivexError)?;

		let type_tag = object::typetag::into_tag(type_.upcast());
		self.hive
			.node(description_handle)
			.set_value::<&str>(
				SetValueFlags::empty(),
				c"Type",
				hivex::value::Value::Dword(type_tag),
			)
			.map_err(HivexError)?;

		// Create elements
		let elements_handle = object.node_add_child(c"Elements").map_err(HivexError)?;
		let elements = self.hive.node(elements_handle);

		Ok(Object {
			type_,
			elements,
			uuid,
		})
	}
}

#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ObjectHandle(NodeHandle);

#[derive(Debug, Display, Error)]
pub enum ObjectRetrievalError {
	#[display = "Missing type information"]
	MissingType,
	#[display = "Type information is not valid"]
	InvalidType,
	#[display = "Missing `Elements` node"]
	MissingElements,
	#[display = "Invalid UUID"]
	Uuid,
}

#[derive(Debug, Display, Error)]
pub enum UuidRetrievalError {
	#[display = "Failed to get object UUID"]
	HiveRetrieval,
	#[display = "Invalid UUID"]
	Uuid,
}

#[derive(Debug, Display, Error)]
#[display = "Registry manipulation error: {0}"]
pub struct HivexError(std::io::Error);
