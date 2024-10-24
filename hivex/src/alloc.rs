//! LibC allocator compatibility proxy

use {
	allocator_api2::alloc::{AllocError, Allocator},
	std::{
		alloc::{GlobalAlloc, Layout},
		ptr::NonNull,
	},
};

/// Proxy for [`libc_alloc::LibcAlloc`] to use with [`allocator_api2`]
#[derive(Clone, Default)]
#[repr(transparent)]
pub struct LibcAlloc;

unsafe impl Allocator for LibcAlloc {
	fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
		let size = layout.size();
		let ptr = unsafe { libc_alloc::LibcAlloc.alloc(layout) };
		if ptr.is_null() {
			return Err(AllocError);
		}

		let slice = unsafe { std::slice::from_raw_parts_mut(ptr, size) };

		unsafe { Ok(NonNull::new_unchecked(&mut *slice)) }
	}

	unsafe fn deallocate(&self, ptr: std::ptr::NonNull<u8>, layout: std::alloc::Layout) {
		unsafe { libc_alloc::LibcAlloc.dealloc(ptr.as_ptr(), layout) }
	}
}
