//! WimLib is made to support both UTF-8 and UTF-16, depending on platform
//!
//! There are special strings for WimLib's use.

use {
	derive_more::{Display, Error},
	std::marker::PhantomData,
};

cfg_if::cfg_if!(if #[cfg(any(windows, doc))] {
	mod utf16;

	pub use utf16::TStr;
	pub(crate) use utf16::ItemType;

	/// Create a new [`crate::string::TStr`] from literal
	#[macro_export]
	macro_rules! tstr {
		($text:expr) => {
			::widestring::widecstr!($text)
		};
	}
} else {
	mod utf8;

	pub use utf8::TStr;
	pub(crate) use utf8::ItemType;

	/// Create a new [`crate::string::TStr`] from literal
	#[macro_export]
	macro_rules! tstr {
		($text:expr) => {{
			let ptr = concat!($text, "\0").as_ptr();
			let cstr = unsafe { ::std::ffi::CStr::from_ptr(ptr as *const std::os::raw::c_char) };
			$crate::string::TStr::from_impl(cstr)
		}};
	}
});

impl Default for &TStr {
	fn default() -> Self {
		tstr!("")
	}
}

/// Extension primarily for [`TStr`] to make optional type into a nullable
/// pointer
pub trait OptionalAsFfiExt {
	/// From optional self as nullabe pointer
	fn as_nullable_ptr(&self) -> *const ItemType;
}

impl OptionalAsFfiExt for Option<&TStr> {
	fn as_nullable_ptr(&self) -> *const ItemType {
		match self {
			Some(value) => value.as_ptr(),
			None => std::ptr::null(),
		}
	}
}

/// [`TStr`] represented as a thin pointer with its length information stripped
#[repr(transparent)]
pub struct ThinTStr<'a> {
	inner: *const ItemType,
	_borrow: PhantomData<&'a TStr>,
}

impl<'a> From<&'a TStr> for ThinTStr<'a> {
	fn from(value: &'a TStr) -> Self {
		Self::new(value)
	}
}

impl<'a> ThinTStr<'a> {
	/// Create a new [`ThinTStr`]
	pub fn new(path: &'a TStr) -> Self {
		Self {
			inner: path.as_ptr(),
			_borrow: PhantomData,
		}
	}

	/// Create an array of [`ThinTStr`] from [`TStr`]
	pub fn array<const N: usize>(paths: [&'a TStr; N]) -> [Self; N] {
		paths.map(Self::new)
	}
}

/// Failed
#[derive(Clone, Copy, Display, Debug, Error)]
pub struct StringCastError;
