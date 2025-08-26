use {
	beef::lean::Cow,
	derive_more::{Display, Error},
	std::{
		ffi::{c_char, CStr, CString, FromBytesWithNulError},
		fmt::{Debug, Display},
		path::Path,
		str::Utf8Error,
	},
};

pub(crate) type ItemType = c_char;

/// WimLib's Unicode C string wrapper
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct TStr {
	pub(super) inner: CStr,
}

impl TStr {
	/// Create a [`TStr`] reference from its underlying
	/// implementation
	pub const fn from_impl(cstr: &CStr) -> &Self {
		unsafe { &*(cstr as *const CStr as *const Self) }
	}

	/// Get inner pointer
	pub const fn as_ptr(&self) -> *const c_char {
		self.inner.as_ptr()
	}

	/// Create from codepoint from nul-terminated UTF-8 slice
	///
	/// Refer to [`CStr::from_bytes_with_nul`]
	pub fn from_slice(slice: &[u8]) -> Result<&Self, impl std::error::Error> {
		if let Err(e) = std::str::from_utf8(slice) {
			return Err(FromSliceError::Unicode(e));
		}

		let cstr = CStr::from_bytes_with_nul(slice).map_err(FromSliceError::Nul)?;
		Ok(Self::from_impl(cstr))
	}

	/// Create a boxed TStr from a boxed CString
	fn from_boxed_cstring(b: Box<CString>) -> Box<TStr> {
		// SAFETY: Box<T> and Box<U> have identical layout because of #[repr(transparent)]
		unsafe { Box::from_raw(Box::into_raw(b) as *mut TStr) }
	}

	/// Create a boxed TStr from Rust str
	pub fn from_str<S: AsRef<str>>(s: S) -> Result<Box<TStr>, Box<dyn std::error::Error>> {
		let boxed_cstring = CString::new(s.as_ref())?.into_boxed_cstring();
		Ok(Self::from_boxed_cstring(boxed_cstring))
	}

	/// Create a boxed TStr from Path
	pub fn from_path<P: AsRef<Path>>(path: P) -> Result<Box<TStr>, Box<dyn std::error::Error>> {
		Self::from_str(path.as_ref().to_string_lossy())
	}
	
	/// Wrap a raw string
	///
	/// # Safety
	/// - Refer to [`CStr::from_ptr`]
	/// - Additionally, the string has to be valid UTF-8
	pub unsafe fn from_ptr<'a>(ptr: *const c_char) -> &'a Self {
		let cstr = unsafe { CStr::from_ptr(ptr) };
		Self::from_impl(cstr)
	}

	/// Wrap a raw string, which may be `NULL` into optional string
	///
	/// # Safety
	/// - Refer to [`Self::from_ptr`]
	/// - Pointer can be `NULL`, then [`None`] is returned
	pub unsafe fn from_ptr_optional<'a>(ptr: *const c_char) -> Option<&'a Self> {
		(!ptr.is_null()).then(|| unsafe { Self::from_ptr(ptr) })
	}

	/// Cast to Rust string
	pub fn to_str(&self) -> Cow<str> {
		self.as_str().into()
	}

	/// Get the lenght of string
	pub const fn len(&self) -> usize {
		self.inner.to_bytes().len()
	}

	/// Is string empty?
	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Get iterator of [`char`]
	pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
		self.as_str().chars()
	}

	fn as_str(&self) -> &str {
		// SAFETY: Library promises us UTF-8
		let string = unsafe { &*(self as *const Self as *const str) };
		&string[0..string.len() - 1]
	}
}

#[derive(Clone, Debug, Display, Error)]
pub enum FromSliceError {
	Unicode(Utf8Error),
	Nul(FromBytesWithNulError),
}

impl Debug for TStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(self.as_str(), f)
	}
}

impl Display for TStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(self.as_str(), f)
	}
}
