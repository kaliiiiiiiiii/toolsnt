//! Registry hive value wrappers and descriptions

use {
	crate::{
		sys,
		utils::{boxed_slice_from_ffi, boxed_str_from_ffi, check_pointer_null, check_status_zero},
		BorrowedHive, LibCBox,
	},
	std::{
		borrow::Cow,
		ffi::{CStr, CString},
		str::Utf8Error,
	},
};

/// A handle to a value
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(transparent)]
pub struct ValueHandle(pub(crate) sys::hive_value_h);

pub struct SelectedValue<'hive> {
	pub(crate) hive: BorrowedHive<'hive>,
	pub(crate) handle: ValueHandle,
}

impl<'hive> SelectedValue<'hive> {
	/// Get registry hive from which this value is selected
	pub fn hive(&self) -> BorrowedHive<'hive> {
		self.hive.clone()
	}

	/// Get a handle used for selecting in registry by in this value
	pub const fn handle(&self) -> ValueHandle {
		self.handle
	}

	/// Return the key of a [`Value`]
	///
	/// Note that this function can return a zero-length string. In the context
	/// of Windows Registries, this means that this value is the default key for
	/// this node in the tree. This is usually written as `"@"`.
	pub fn key(&self) -> std::io::Result<LibCBox<str>> {
		unsafe {
			let raw_ptr = self.hive.as_handle();
			let data = sys::hivex_value_key(raw_ptr, self.handle.0);
			check_pointer_null(data)?;

			let len = sys::hivex_value_key_len(raw_ptr, self.handle.0);
			Ok(boxed_str_from_ffi(data, len))
		}
	}

	/// Return the lenght and data type of [`Value`]
	///
	/// You can use [`Self::value_get`] if you know the type in advance.
	pub fn type_of(&self) -> Option<ValueTypeRet> {
		let mut ffi_type = 0;
		let mut len = 0;
		let result = unsafe {
			sys::hivex_value_type(
				self.hive.as_handle(),
				self.handle.0,
				&mut ffi_type,
				&mut len,
			)
		};

		let ty = ValueType::from_ffi(ffi_type)?;
		(result == 0).then_some(ValueTypeRet { ty, len })
	}

	/// Return [`ValueBytesRet`] of [`Value`]. The byte slice should be
	/// interpreted according to its type.
	pub fn raw(&self) -> std::io::Result<RawValue> {
		let mut ffi_type = 0;
		let mut len = 0;

		let data = unsafe {
			sys::hivex_value_value(
				self.hive.as_handle(),
				self.handle.0,
				&mut ffi_type,
				&mut len,
			)
		};

		check_pointer_null(data)?;

		let ty =
			ValueType::from_ffi(ffi_type).ok_or_else(|| std::io::Error::other("Invalid type"))?;

		let bytes = unsafe { boxed_slice_from_ffi(data.cast(), len) };

		Ok(RawValue { ty, data: bytes })
	}

	/// Get a [`Value`] by its handle.
	pub fn get(&self) -> std::io::Result<Value<LibCBox<str>>> {
		let mut ffi_ty = 0;
		let mut len = 0;
		let status = unsafe {
			sys::hivex_value_type(self.hive.as_handle(), self.handle.0, &mut ffi_ty, &mut len)
		};

		check_status_zero(status)?;

		let ty = ValueType::from_ffi(ffi_ty).expect("Expected valid type");

		let value = match ty {
			ValueType::None => Value::None,
			ValueType::Sz | ValueType::ExpandSz | ValueType::Link => {
				Value::Sz(self.downcast_string()?)
			}
			ValueType::MultiSz => Value::MultiSz(self.downcast_multiple_strings()),
			ValueType::Binary
			| ValueType::ResourceList
			| ValueType::FullResourceDescriptor
			| ValueType::ResourceRequirementsList => Value::Binary(self.downcast_binary()?),
			ValueType::Dword | ValueType::DwordBe => Value::Dword(self.downcast_dword()),
			ValueType::Qword => Value::Qword(self.downcast_qword()),
		};

		Ok(value)
	}

	/// Expecting string-like value, extract it or error out
	fn downcast_string(&self) -> std::io::Result<LibCBox<str>> {
		unsafe {
			let data = sys::hivex_value_string(self.hive.as_handle(), self.handle.0);
			check_pointer_null(data)?;
			let c_str_len = CStr::from_ptr(data).to_bytes_with_nul().len();
			Ok(boxed_str_from_ffi(data, c_str_len))
		}
	}

	/// Expected binary value, extract it or error out
	fn downcast_binary(&self) -> std::io::Result<LibCBox<[u8]>> {
		let mut _ffi_ty = 0;
		let mut len = 0;

		unsafe {
			let data = sys::hivex_value_value(
				self.hive.as_handle(),
				self.handle.0,
				&mut _ffi_ty,
				&mut len,
			);
			check_pointer_null(data)?;
			Ok(boxed_slice_from_ffi(data.cast(), len))
		}
	}

	/// Expected multiple strings value, extract it or error out
	fn downcast_multiple_strings(&self) -> Box<[LibCBox<str>]> {
		let mut strings = vec![];

		unsafe {
			let mut array_of_strings =
				sys::hivex_value_multiple_strings(self.hive.as_handle(), self.handle.0);

			loop {
				let str_ptr = array_of_strings.read();

				// Terminated by null pointer → end of slice
				if str_ptr.is_null() {
					break;
				}

				let len = CStr::from_ptr(str_ptr).to_bytes_with_nul().len();
				let str = boxed_str_from_ffi(str_ptr, len);
				strings.push(str);
				array_of_strings = array_of_strings.add(1);
			}
		}

		strings.into_boxed_slice()
	}

	/// Expected LE or BE DWORD int, extract and convert or error out
	pub fn downcast_dword(&self) -> u32 {
		unsafe { sys::hivex_value_dword(self.hive.as_handle(), self.handle.0) as u32 }
	}

	/// Expected QWORD int, extract or error out
	pub fn downcast_qword(&self) -> u64 {
		unsafe { sys::hivex_value_qword(self.hive.as_handle(), self.handle.0) as u64 }
	}
}

/// [`SelectedValue::type_of`] return structure
///
/// Contains value type and underlying raw data size
#[derive(Clone, Copy, Debug)]
pub struct ValueTypeRet {
	/// Value type
	pub ty: ValueType,
	/// Lenght of raw data
	pub len: usize,
}

/// [`SelectedValue::raw`] return structure
pub struct RawValue {
	/// Value type
	pub ty: ValueType,
	/// Underlying value raw data
	pub data: LibCBox<[u8]>,
}

/// Value type tag
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[non_exhaustive]
pub enum ValueType {
	/// No defined value type
	None = sys::hive_type_hive_t_REG_NONE as _,
	/// A Windows string (encoding is unknown, but often UTF16-LE)
	Sz = sys::hive_type_hive_t_REG_SZ as _,
	/// A Windows string that contains %env% (environment variable expansion)
	ExpandSz = sys::hive_type_hive_t_REG_EXPAND_SZ as _,
	/// A blob of binary
	Binary = sys::hive_type_hive_t_REG_BINARY as _,
	/// DWORD (32 bit integer), little endian
	Dword = sys::hive_type_hive_t_REG_DWORD as _,
	/// DWORD (32 bit integer), big endian
	DwordBe = sys::hive_type_hive_t_REG_DWORD_BIG_ENDIAN as _,
	/// Symbolic link to another part of the registry tree
	Link = sys::hive_type_hive_t_REG_LINK as _,
	/// Multiple Windows strings
	MultiSz = sys::hive_type_hive_t_REG_MULTI_SZ as _,
	/// Resource list
	ResourceList = sys::hive_type_hive_t_REG_RESOURCE_LIST as _,
	/// Resource descriptor
	FullResourceDescriptor = sys::hive_type_hive_t_REG_FULL_RESOURCE_DESCRIPTOR as _,
	/// Resouce requirements list
	ResourceRequirementsList = sys::hive_type_hive_t_REG_RESOURCE_REQUIREMENTS_LIST as _,
	/// QWORD (64 bit integer), unspecified endianness but usually little endian
	Qword = sys::hive_type_hive_t_REG_QWORD as _,
}

impl ValueType {
	pub(crate) fn from_ffi(n: sys::hive_type) -> Option<Self> {
		(n <= Self::Qword as sys::hive_type).then(|| unsafe { std::mem::transmute(n as u8) })
	}
}

/// Converted registry value
///
/// # Data representation note
/// Windows RegEdit itself shows [`Value::ResourceList`],
/// [`Value::FullResourceDescriptor`] and [`Value::ResourceRequirementsList`] as
/// plain bytes. I may try to reverse-engineer some inner structure and then
/// support that. Until then, enjoy your bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value<Str> {
	/// No defined value type
	None,
	/// A string
	Sz(Str),
	/// A string that contains %env% (environment variable expansion)
	ExpandSz(Str),
	/// Blob of bytes
	Binary(LibCBox<[u8]>),
	/// 32 bit LE integer
	Dword(u32),
	/// 32 bit BE integer (should not appear in returned Value)
	DwordBe(u32),
	/// Symbolic link to another part of the registry tree
	Link(Str),
	/// Multiple strings
	MultiSz(Box<[Str]>),
	/// Resource list
	ResourceList(LibCBox<[u8]>),
	/// Resource descriptor
	FullResourceDescriptor(LibCBox<[u8]>),
	/// Resource requirements list
	ResourceRequirementsList(LibCBox<[u8]>),
	/// 64 bit integer
	Qword(u64),
}

impl<Str> From<u32> for Value<Str> {
	fn from(val: u32) -> Self {
		Self::Dword(val)
	}
}

impl<Str> From<u64> for Value<Str> {
	fn from(value: u64) -> Self {
		Self::Qword(value)
	}
}

impl<Str: ValueString> From<Str> for Value<Str> {
	fn from(value: Str) -> Self {
		Self::Sz(value)
	}
}

impl<Str> Value<Str> {
	/// Get a type of value
	pub const fn type_of(&self) -> ValueType {
		match self {
			Value::None => ValueType::None,
			Value::Sz(_) => ValueType::Sz,
			Value::ExpandSz(_) => ValueType::ExpandSz,
			Value::Binary(_) => ValueType::Binary,
			Value::Dword(_) => ValueType::Dword,
			Value::DwordBe(_) => ValueType::DwordBe,
			Value::Link(_) => ValueType::Link,
			Value::MultiSz(_) => ValueType::MultiSz,
			Value::ResourceList(_) => ValueType::ResourceList,
			Value::FullResourceDescriptor(_) => ValueType::FullResourceDescriptor,
			Value::ResourceRequirementsList(_) => ValueType::ResourceRequirementsList,
			Value::Qword(_) => ValueType::Qword,
		}
	}
}

/// A trait to be used for user-provided string values
pub trait ValueString {
	/// Make the string into a nul-terminated one
	///
	/// If it already is NUL-terminated, no need to
	/// copy anything, so that's why [`Cow`] is used.
	fn into_c_string<'a>(self) -> Cow<'a, CStr>
	where
		Self: 'a;

	/// Get string lenght without NUL terminator
	fn c_strlen(&self) -> usize;

	/// Convert string to UTF-16 encoded byte sequence
	fn to_utf16(&self) -> Result<Box<[u16]>, Utf8Error>;
}

impl ValueString for &str {
	fn into_c_string<'a>(self) -> Cow<'a, CStr>
	where
		Self: 'a,
	{
		ValueString::into_c_string(self.as_bytes())
	}

	fn c_strlen(&self) -> usize {
		ValueString::c_strlen(&self.as_bytes())
	}

	fn to_utf16(&self) -> Result<Box<[u16]>, Utf8Error> {
		Ok(self.encode_utf16().collect())
	}
}

impl ValueString for &[u8] {
	fn into_c_string<'a>(self) -> Cow<'a, CStr>
	where
		Self: 'a,
	{
		match CStr::from_bytes_until_nul(self) {
			Ok(x) => Cow::Borrowed(x),
			Err(_) => {
				let cstring = CString::new(self);
				match cstring {
					Ok(cstring) => Cow::Owned(cstring),
					Err(_) => unreachable!("This branch exists solely for converting non-NUL terminated string. NUL found."),
				}
			}
		}
	}

	fn c_strlen(&self) -> usize {
		match CStr::from_bytes_until_nul(self) {
			Ok(x) => x.to_bytes().len(),
			Err(_) => self.len(),
		}
	}

	fn to_utf16(&self) -> Result<Box<[u16]>, Utf8Error> {
		let utf8 = std::str::from_utf8(self)?;
		Ok(utf8.encode_utf16().collect())
	}
}

impl ValueString for &CStr {
	fn into_c_string<'a>(self) -> Cow<'a, CStr>
	where
		Self: 'a,
	{
		Cow::Borrowed(self)
	}

	fn c_strlen(&self) -> usize {
		self.to_bytes().len()
	}

	fn to_utf16(&self) -> Result<Box<[u16]>, Utf8Error> {
		ValueString::to_utf16(&self.to_bytes())
	}
}

impl ValueString for CString {
	fn into_c_string<'a>(self) -> Cow<'a, CStr>
	where
		Self: 'a,
	{
		Cow::Owned(self)
	}

	fn c_strlen(&self) -> usize {
		self.as_bytes().len()
	}

	fn to_utf16(&self) -> Result<Box<[u16]>, Utf8Error> {
		// Reuse implementation for &CStr
		self.as_c_str().to_utf16()
	}
}

impl ValueString for Cow<'_, CStr> {
	fn into_c_string<'a>(self) -> Cow<'a, CStr>
	where
		Self: 'a,
	{
		self
	}

	fn c_strlen(&self) -> usize {
		self.to_bytes().c_strlen()
	}

	fn to_utf16(&self) -> Result<Box<[u16]>, Utf8Error> {
		self.to_bytes().to_utf16()
	}
}
