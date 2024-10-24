use {
	crate::{alloc::LibCAlloc, LibCBox},
	std::ptr::NonNull,
};

/// Get last OS error
///
/// Wrapper for [`std::io::Error::last_os_error`]
pub fn last_os_error() -> std::io::Error {
	std::io::Error::last_os_error()
}

/// Check if pointer is null. If it is, return last OS error.
pub fn check_pointer_null<T>(ptr: *mut T) -> std::io::Result<()> {
	if ptr.is_null() {
		Err(last_os_error())
	} else {
		Ok(())
	}
}

/// Check if status integer is zero. If it is, return last OS error.
pub fn check_status_zero(status: std::ffi::c_int) -> std::io::Result<()> {
	if status == 0 {
		Ok(())
	} else {
		Err(last_os_error())
	}
}

/// Check if handle value is zero. If it is, return last OS error,
/// otherwise wrap it in handle wrapper.
pub fn wrap_handle<T>(value: usize, wrap: impl Fn(usize) -> T) -> std::io::Result<T> {
	if value == 0 {
		Err(last_os_error())
	} else {
		Ok(wrap(value))
	}
}

/// Construct LibC boxed slice from »raw parts«
///
/// # Safety
/// - Upholds same safety constraints as
///   [`allocator_api2::vec::Vec::from_raw_parts`]
///     - Except the data pointer can be null
///
/// # Empty slice behaviour
/// - If data is null and length is zero → provide zero length and dangling
///   pointer as the value
/// - If data is null and lenght is non-zero → weird library behaviour, panic
/// - If data is not null and lenght is zero → just wrap it
pub unsafe fn boxed_slice_from_ffi<T>(data: *mut T, len: usize) -> LibCBox<[T]> {
	let data = match (len == 0, data.is_null()) {
		(true, true) => NonNull::dangling().as_ptr(),
		(false, true) => {
			panic!("Hivex library returned null pointer to slice while having non-zero length")
		}
		(_, false) => data,
	};

	unsafe {
		let vector = allocator_api2::vec::Vec::from_raw_parts_in(data, len, len, LibCAlloc);
		vector.into_boxed_slice()
	}
}

/// Construct LibC boxed UTF-8 string from »raw parts«
///
/// # Safety
/// - Refer to [`boxed_slice_from_ffi`]
///
/// # Empty string behaviour
/// - Refer to [`boxed_slice_from_ffi`]
pub unsafe fn boxed_str_from_ffi(data: *mut std::ffi::c_char, len: usize) -> LibCBox<str> {
	unsafe {
		let data = boxed_slice_from_ffi(data, len);
		LibCBox::from_raw_in(LibCBox::into_raw(data) as *mut str, LibCAlloc)
	}
}
