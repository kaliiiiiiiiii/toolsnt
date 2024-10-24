//! Things regarding nodes

use {
	crate::{
		sys,
		utils::{
			boxed_slice_from_ffi, boxed_str_from_ffi, check_pointer_null, check_status_zero,
			last_os_error, wrap_handle,
		},
		value::{Value, ValueHandle, ValueString, ValueType},
		BorrowedHive, LibCBox, SetValueFlags,
	},
	std::{ffi::CStr, mem::size_of, ops::Deref, ptr::addr_of},
	time::PrimitiveDateTime,
};

/// A node handle
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct NodeHandle(pub(crate) sys::hive_node_h);

/// Selected node
pub struct SelectedNode<'hive> {
	pub(crate) hive: BorrowedHive<'hive>,
	pub(crate) handle: NodeHandle,
}

impl SelectedNode<'_> {
	/// Get registry hive from which this node is selected
	pub fn hive(&self) -> BorrowedHive {
		self.hive.clone()
	}

	/// Get a handle used for selecting in registry by in this node
	pub const fn handle(&self) -> NodeHandle {
		self.handle
	}

	/// Return the name of the node
	///
	/// Note that the name of the root node is a dummy, such as `$$$PROTO.HIV`
	/// (other names are possible: it seems to depend on the tool or program
	/// that created the hive in the first place). You can only know the "real"
	/// name of the root node by knowing which registry file this hive
	/// originally comes from, which is knowledge that is outside the scope of
	/// this library.
	pub fn name(&self) -> std::io::Result<LibCBox<str>> {
		unsafe {
			let raw_ptr = self.hive.as_handle();
			let data = sys::hivex_node_name(raw_ptr, self.handle.0);
			check_pointer_null(data)?;

			let len = sys::hivex_node_name_len(raw_ptr, self.handle.0);
			Ok(boxed_str_from_ffi(data, len))
		}
	}

	/// Return the modification time of the node
	pub fn timestamp(&self) -> PrimitiveDateTime {
		let raw = unsafe { sys::hivex_node_timestamp(self.hive.as_handle(), self.handle.0) };
		crate::win_filetime_to_primitive_datetime(raw)
	}

	/// Return an owned slice of [`NodeHandle`] which are the subkeys (children)
	/// of node.
	pub fn children(&self) -> LibCBox<[NodeHandle]> {
		unsafe {
			let raw_ptr = self.hive.as_handle();
			let data = sys::hivex_node_children(raw_ptr, self.handle.0).cast();
			let len = sys::hivex_node_nr_children(raw_ptr, self.handle.0);
			boxed_slice_from_ffi(data, len)
		}
	}

	/// Return the child of node with the name name, if it exists
	pub fn get_child(&self, name: &str) -> Option<NodeHandle> {
		// Yes. This method has implementation in the library.
		// But it does the same thing so we are saving overhead of
		// using C strings.
		//
		// If the wrapped library comes up with better solution, use it.

		let searched_name = name.trim_end_matches('\0');
		let children = self.children();
		children
			.iter()
			.find(|&&child| {
				let child_name = self
					.hive
					.node(child)
					.name()
					.expect("The node was in child list, the name should exist");

				child_name.deref() == searched_name
			})
			.copied()
	}

	/// Return the parent of node
	///
	/// The parent pointer of the root node in registry files that we have
	/// examined seems to be invalid, and so this function will return an error
	/// if called on the root node.
	pub fn parent(&self) -> std::io::Result<NodeHandle> {
		let parent = unsafe { sys::hivex_node_parent(self.hive.as_handle(), self.handle.0) };
		wrap_handle(parent, NodeHandle)
	}

	/// Return the array of [`Value`] attached to this node
	pub fn values(&self) -> LibCBox<[ValueHandle]> {
		unsafe {
			let raw_ptr = self.hive.as_handle();
			let data = sys::hivex_node_values(raw_ptr, self.handle.0).cast();
			let len = sys::hivex_node_nr_values(raw_ptr, self.handle.0);
			boxed_slice_from_ffi(data, len)
		}
	}

	/// Return the value attached to this node which has the name `key`, if it
	/// exists.
	///
	/// Note that to get the default key, you should pass the empty string here.
	/// The default key is often written `"@"`, but inside hives that has no
	/// meaning and won't give you the default key.
	pub fn get_value(&self, key: impl ValueString) -> std::io::Result<ValueHandle> {
		let c_str_key = key.into_c_string();
		let handle = unsafe {
			sys::hivex_node_get_value(self.hive.as_handle(), self.handle.0, c_str_key.as_ptr())
		};

		wrap_handle(handle, ValueHandle)
	}

	/// Add a new child node named `name` to the existing node `parent`.
	///
	/// The new child initially has no subnodes and contains no keys or values.
	/// The sk-record (security descriptor) is inherited from the parent.
	pub fn node_add_child(&self, name: impl ValueString) -> std::io::Result<NodeHandle> {
		let c_name = name.into_c_string();
		let result = unsafe {
			sys::hivex_node_add_child(self.hive.as_handle(), self.handle.0, c_name.as_ptr())
		};

		if result != 0 {
			Ok(NodeHandle(result))
		} else {
			Err(last_os_error())
		}
	}

	/// Delete the node `node`.
	///
	/// All values at the node and all subnodes are deleted (recursively).
	///
	/// The node handle and the handles of all subnodes become invalid. You
	/// cannot delete the root node.
	pub fn delete(self) -> std::io::Result<()> {
		let status = unsafe { sys::hivex_node_delete_child(self.hive.as_handle(), self.handle.0) };
		check_status_zero(status)
	}

	/// Set a value under `node` with name `key`
	///
	/// There are no `flags`` specified at this time, use
	/// [`SetValueFlags::empty`]
	pub fn set_value<Str: ValueString>(
		&self,
		flags: SetValueFlags,
		key: impl ValueString,
		value: Value<Str>,
	) -> std::io::Result<()> {
		let c_key = key.into_c_string();
		let ty = value.type_of();

		// In most cases, just get pointer to the data
		let (data, len) = match value {
			Value::None => (std::ptr::null(), 0),
			Value::Binary(x)
			| Value::ResourceList(x)
			| Value::FullResourceDescriptor(x)
			| Value::ResourceRequirementsList(x) => (x.as_ptr(), x.len()),
			Value::Dword(x) => ((&raw const x).cast(), size_of::<i32>()),
			Value::DwordBe(x) => (addr_of!(x).cast(), size_of::<i32>()),
			Value::Qword(x) => (addr_of!(x).cast(), size_of::<i64>()),

			// Requires special treatment, use separate functions:
			Value::Sz(x) | Value::ExpandSz(x) | Value::Link(x) => {
				return self.set_value_string(flags, &*c_key, ty, x);
			}
			Value::MultiSz(x) => return self.set_value_multistring(flags, &c_key, ty, &x),
		};

		// Construct structure for setting value
		let set_value = sys::hive_set_value {
			key: c_key.as_ptr().cast_mut().cast(),
			t: ty as u32,
			len,
			value: data.cast_mut().cast(),
		};

		let status = unsafe {
			sys::hivex_node_set_value(
				self.hive.as_handle(),
				self.handle.0,
				&set_value,
				flags.bits(),
			)
		};

		check_status_zero(status)
	}

	/// Set value to a string value (that being [`Value::Sz`],
	/// [`Value::ExpandSz`] and [`Value::Link`])
	pub fn set_value_string(
		&self,
		flags: SetValueFlags,
		key: impl ValueString,
		ty: ValueType,
		bytes: impl ValueString,
	) -> std::io::Result<()> {
		debug_assert!(
			matches!(ty, ValueType::Sz | ValueType::ExpandSz | ValueType::Link),
			"Provided type should be string-like",
		);

		let c_key = key.into_c_string();

		// Convert bytes to UTF-16 string
		let mut utf16 = bytes.to_utf16().map_err(|_| {
			std::io::Error::new(
				std::io::ErrorKind::InvalidData,
				"The string is not valid Unicode",
			)
		})?;

		let set_value = sys::hive_set_value {
			key: c_key.as_ptr().cast_mut().cast(),
			t: ty as u32,
			len: utf16.len() * size_of::<u16>(),
			value: utf16.as_mut_ptr().cast(),
		};

		let status = unsafe {
			sys::hivex_node_set_value(
				self.hive.as_handle(),
				self.handle.0,
				&set_value,
				flags.bits(),
			)
		};

		check_status_zero(status)
	}

	/// Set value of type `MultiSz`
	fn set_value_multistring(
		&self,
		flags: SetValueFlags,
		key: &CStr,
		ty: ValueType,
		data: &[impl ValueString],
	) -> std::io::Result<()> {
		let mut utf16_data = vec![];
		for string in data {
			utf16_data.extend_from_slice(&string.to_utf16().unwrap());
			if utf16_data.last() != Some(&0) {
				utf16_data.push(0);
			}
		}

		let len = utf16_data.len() * size_of::<u16>();

		let set_value = sys::hive_set_value {
			key: key.as_ptr().cast_mut().cast(),
			t: ty as u32,
			len,
			value: utf16_data.as_ptr().cast_mut().cast(),
		};

		let status = unsafe {
			sys::hivex_node_set_value(
				self.hive.as_handle(),
				self.handle.0,
				&set_value,
				flags.bits(),
			)
		};

		check_status_zero(status)
	}
}
