use {
	beef::lean::Cow,
	std::fmt::{Debug, Display},
	std::path::Path,
	widestring::{error::NulError, U16CStr, U16CString},
};

pub(crate) type ItemType = u16;

/// WimLib's Unicode C string wrapper
#[derive(PartialEq, Eq)]
#[repr(transparent)]
pub struct TStr {
	pub(super) inner: U16CStr,
}

impl TStr {
	/// Create a [`TStr`] reference from its underlying
	/// implementation
	pub const fn from_impl(cstr: &U16CStr) -> &Self {
		unsafe { &*(cstr as *const U16CStr as *const Self) }
	}

	/// Get inner pointer
	pub const fn as_ptr(&self) -> *const u16 {
		self.inner.as_ptr()
	}

	/// Create from codepoint from nul-terminated UTF-8 slice
	///
	/// Refer to [`U16CStr::from_slice`]
	pub fn from_slice(slice: &[u16]) -> Result<&Self, impl std::error::Error> {
		let cstr = U16CStr::from_slice(slice)?;
		Ok::<_, NulError<_>>(Self::from_impl(cstr))
	}

	fn from_boxed_ucstr(b: Box<U16CStr>) -> Box<TStr> {
		// SAFETY: `TStr` is #[repr(transparent)] over `U16CStr`.
		// Box<T> and Box<U> have identical layout in memory.
		unsafe { Box::from_raw(Box::into_raw(b) as *mut TStr) }
	}

	/// Create a boxed TStr from a Rust `&str`
	pub fn from_str<S: AsRef<str>>(s: S) -> Result<Box<TStr>, Box<dyn std::error::Error>> {
		let s_ref = s.as_ref();
		let boxed_ucstr = U16CString::from_str(s_ref)?.into_boxed_ucstr();
		Ok(TStr::from_boxed_ucstr(boxed_ucstr))
	}

	/// create boxed Tstr from bytes
	pub fn from_bytes<B: AsRef<[u8]>>(bytes: B) -> Result<Box<TStr>, Box<dyn std::error::Error>> {
		Self::from_str(std::str::from_utf8(bytes.as_ref())?)
	}

	/// create boxed tstr from Path
	pub fn from_path<P>(path: P) -> Result<Box<TStr>, Box<dyn std::error::Error>>
	where
		P: AsRef<Path>,
	{
		let path_ref = path.as_ref();
		Self::from_str(path_ref.to_string_lossy())
	}

	/// Wrap a raw string
	///
	/// # Safety
	/// - Refer to [`U16CStr::from_ptr_str`]
	pub unsafe fn from_ptr<'a>(ptr: *const u16) -> &'a Self {
		let cstr = unsafe { U16CStr::from_ptr_str(ptr) };
		Self::from_impl(cstr)
	}

	/// Wrap a raw string, which may be `NULL` into optional string
	///
	/// # Safety
	/// - Refer to [`Self::from_ptr`]
	/// - Pointer can be `NULL`, then [`None`] is returned
	pub unsafe fn from_ptr_optional<'a>(ptr: *const u16) -> Option<&'a Self> {
		(!ptr.is_null()).then(|| unsafe { Self::from_ptr(ptr) })
	}

	/// Cast to Rust string (lossy)
	pub fn to_str(&self) -> Cow<str> {
		self.inner.to_string_lossy().into()
	}

	/// Get the lenght of string
	pub const fn len(&self) -> usize {
		self.inner.len()
	}

	/// Is string empty?
	pub const fn is_empty(&self) -> bool {
		self.len() == 0
	}

	/// Get iterator of [`char`]
	pub fn chars(&self) -> impl Iterator<Item = char> + '_ {
		self.inner.chars_lossy()
	}
}

impl Debug for TStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Debug::fmt(&self.inner.display(), f)
	}
}

impl Display for TStr {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.inner.display(), f)
	}
}
