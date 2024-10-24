//! # Hivex
//!
//! _FFI bindings for the [Hivex] C library_
//!
//! Hivex is a library for reading and manipulating Windows NT registry hives.
//! This crate attempts to idiomatically wrap this library for usage in Rust.
//!
//! Most of the documentation, which isn't a Rust-specific concept like
//! [`SelectedNode`][`node::SelectedNode`] is taken and adapted from the [Hivex]
//! documentation.
//!
//! ## Core concepts
//! - [Hive][`Hive`]: Windows Registry database file. These do not have to
//!   correspond to `HKEY`s in the tree. [Learn more in Microsoft docs][Hive].
//! - Node: That's what Microsoft calls keys. These contain key-value entries
//!   (ye I know, confusing).
//! - [Value][`value::Value`]: Values associated to keys in nodes. They have
//!   several types
//! - Handle: An opaque reference to an entity inside hive
//! - `Selected`{[`Node`][`node::SelectedNode`],
//!   [`Value`][`value::SelectedValue`]}: Convenience wrapper referencing hive
//!   and the entity which allows you to manipulate it
//!
//! ## Implementation notes
//! - Hivex library itself doesn't support creation of new hives. This crate
//!   contains a pre-defined empty registry hive created on Windows NT 10.0
//! - Strings retrieved from the hive will get its NUL-terminator striped by the
//!   bindings
//!
//! ## Warning
//! **Not everything is yet tested. Some data may be saved or retrieved
//! incorrectly and corrupt your system. Proceed with caution.**
//!
//! [Hivex]: https://libguestfs.org/hivex.3.html
//! [Hive]: https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-hives

#![forbid(unsafe_op_in_unsafe_fn)]

pub mod alloc;
pub mod node;
pub mod value;

mod utils;

pub use {alloc::LibCBox, hivex_sys as sys, sys::VERSION};

use {
	node::{NodeHandle, SelectedNode},
	std::{ffi::CStr, io::Write, marker::PhantomData, mem::ManuallyDrop, ops::Deref, path::Path},
	time::PrimitiveDateTime,
	utils::{check_pointer_null, check_status_zero, wrap_handle},
	value::{SelectedValue, ValueHandle, ValueString},
};

/// Empty registry hive template
///
/// Created from BCD template which got wiped
pub static EMPTY_HIVE_TEMPLATE: &[u8] = include_bytes!("../EmptyHive.dat");

bitflags::bitflags! {
	/// Flags for [`Hive::open`]
	#[derive(Clone, Copy, Default, PartialEq, Eq)]
	pub struct OpenFlags: std::ffi::c_int {
		/// Verbose messages
		const VERBOSE = sys::HIVEX_OPEN_VERBOSE as _;

		/// Very verbose messages, suitable for debugging problems
		/// in the library itself.
		///
		/// This is also selected if the `HIVEX_DEBUG` environment
		/// variable is set to 1.
		const DEBUG   = sys::HIVEX_OPEN_DEBUG as _;

		/// Open the hive for writing. If omitted, the hive is read-only.
		const WRITE   = sys::HIVEX_OPEN_WRITE as _;

		/// Open the hive in unsafe mode that enables heuristics to handle
		/// corrupted hives.
		///
		/// This may allow to read or write registry keys / values that appear
		/// intact in an otherwise corrupted hive. Use at your own risk.
		const UNSAFE  = sys::HIVEX_OPEN_UNSAFE as _;
	}

	/// Flags for [`Hive::commit`]
	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
	pub struct CommitFlags: std::ffi::c_int {}

	/// Flags for [`node::SelectedNode::set_value`]
	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
	pub struct SetValueFlags: std::ffi::c_int {}
}

/// Handle to Windows Registry file
#[repr(transparent)]
pub struct Hive(*mut sys::hive_h);
unsafe impl Send for Hive {}

impl Hive {
	/// Creates and opens a new hive file
	pub fn create(path: impl AsRef<Path>, flags: OpenFlags) -> std::io::Result<Self> {
		{
			let mut file = std::fs::File::options()
				.create_new(true)
				.write(true)
				.open(&path)?;

			file.write_all(EMPTY_HIVE_TEMPLATE)?;
		}

		let c_path = path.as_ref().as_os_str().as_encoded_bytes();
		Self::open(c_path, flags)
	}

	/// Opens a hive file for procession
	pub fn open(path: impl ValueString, flags: OpenFlags) -> std::io::Result<Self> {
		let path_c_str = path.into_c_string();
		let handle = unsafe { sys::hivex_open(path_c_str.as_ptr(), flags.bits()) };
		check_pointer_null(handle)?;
		Ok(Self(handle))
	}

	/// Close a hive handle and free all associated resources
	///
	/// Note that any uncommitted writes are *not* committed by this call,
	/// but instead are lost.
	pub fn close(self) -> std::io::Result<()> {
		let status = unsafe { sys::hivex_close(self.as_handle()) };
		check_status_zero(status)
	}

	/// Return root node of the hive. All valid hives must contain a root node.
	pub fn root(&self) -> std::io::Result<NodeHandle> {
		let node = unsafe { sys::hivex_root(self.as_handle()) };
		wrap_handle(node, NodeHandle)
	}

	/// Return the modification time from the header of the hive
	pub fn last_modified(&self) -> PrimitiveDateTime {
		let raw_wintime = unsafe { sys::hivex_last_modified(self.as_handle()) };
		win_filetime_to_primitive_datetime(raw_wintime)
	}

	/// Write changes to the hive.
	///
	/// If `filename` is [`None`], it will overwrite the original file.
	///
	/// `flags` is not currently in use and the only possible value is
	/// [`CommitFlags::empty`]
	pub fn commit(&self, filename: Option<&CStr>, flags: CommitFlags) -> std::io::Result<()> {
		let filename = match filename {
			Some(cstr) => cstr.as_ptr(),
			None => std::ptr::null(),
		};

		let status = unsafe { sys::hivex_commit(self.as_handle(), filename, flags.bits()) };
		check_status_zero(status)
	}

	/// Select a node
	pub const fn node(&self, node: NodeHandle) -> SelectedNode {
		SelectedNode {
			hive: self.borrow(),
			handle: node,
		}
	}

	/// Select a value
	pub const fn value(&self, value: ValueHandle) -> SelectedValue {
		SelectedValue {
			hive: self.borrow(),
			handle: value,
		}
	}

	/// Get inner raw handle
	pub const fn as_handle(&self) -> *mut sys::hive_h {
		self.0
	}

	/// Wrap raw handle
	///
	/// # Disclaimer
	/// - This wrapper after being dropped does close the handle and further
	///   operations on same handle will not be valid.
	pub const fn from_handle(handle: *mut sys::hive_h) -> Self {
		Self(handle)
	}

	/// Create borrowed handle
	///
	/// Doesn't create a memory reference, rather a lifetime-checked
	/// copy.
	pub const fn borrow(&self) -> BorrowedHive<'_> {
		let self_copy = Self::from_handle(self.as_handle());
		BorrowedHive {
			hive: ManuallyDrop::new(self_copy),
			_lifetime: PhantomData,
		}
	}
}

impl Drop for Hive {
	fn drop(&mut self) {
		unsafe { sys::hivex_close(self.as_handle()) };
	}
}

/// Borrowed hive.
///
/// Not a memory reference, contains a copy of hive.
///
/// Used for borrow checking.
#[repr(transparent)]
pub struct BorrowedHive<'a> {
	pub hive: ManuallyDrop<Hive>,
	_lifetime: PhantomData<&'a Hive>,
}

impl Clone for BorrowedHive<'_> {
	fn clone(&self) -> Self {
		Self {
			hive: ManuallyDrop::new(Hive(self.hive.0)),
			_lifetime: PhantomData,
		}
	}
}

impl Deref for BorrowedHive<'_> {
	type Target = Hive;

	fn deref(&self) -> &Self::Target {
		&self.hive
	}
}

/// Convert Windows [File Time](https://learn.microsoft.com/en-us/windows/win32/sysinfo/file-times) to [`PrimitiveDateTime`]
fn win_filetime_to_primitive_datetime(win_time: i64) -> PrimitiveDateTime {
	let epoch_start = PrimitiveDateTime::new(
		time::Date::from_ordinal_date(1601, 1)
			.expect("Start date of Windows Epoch should be valid"),
		time::Time::MIDNIGHT,
	);

	let duration = time::Duration::milliseconds(win_time / 10);
	epoch_start
		.checked_add(duration)
		.expect("This should be a valid date")
}
