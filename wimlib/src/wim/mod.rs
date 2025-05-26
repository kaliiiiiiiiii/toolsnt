//! Most of the interfaces live in here. Manipulating WIMs

pub mod extract;
pub mod info;
pub mod initial;
pub mod modify;
pub mod non_standalone;
pub mod write;

pub use {extract::*, info::*, initial::*, modify::*, non_standalone::*, write::*};

cfg_if! {
	if #[cfg(any(feature = "mount", doc))] {
		pub mod mount;
	}
}

use {
	crate::{progress::ProgressCallback, sys, WimLib},
	cfg_if::cfg_if,
	std::{marker::PhantomData, num::NonZero, ptr::NonNull},
};

/// Index for an image in a WIM
///
/// Zero is considered an invalid value, that's why [`NonZero`]
pub type ImageIndex = NonZero<u32>;

/// All images index
///
/// For selecting all images, using [`Wim::select_all_images`] is prefered.
pub const ALL_IMAGES: ImageIndex = ImageIndex::new(u32::MAX).unwrap();

const fn new_image_index(n: i32) -> Option<ImageIndex> {
	ImageIndex::new(n as u32)
}

/// Structure that contains opaque structure reference representing WIM and
/// an optional progress callback pointer.
pub struct Wim {
	wimstruct: *mut sys::WIMStruct,
	progress_callback: Option<SavedProgressCallback>,
	_wimlib: WimLib,
}

impl Wim {
	/// Select an image. See [`Image`] for details
	///
	/// If you want to select all images, consider using
	/// [`Self::select_all_images`] instead of [`u32::MAX`]
	pub fn select_image(&self, index: ImageIndex) -> Image {
		Image {
			index,
			wimstruct: self.wimstruct,
			_borrows_wim: PhantomData,
		}
	}

	/// Select all images. See [`Image`] for details
	pub fn select_all_images(&self) -> Image {
		Image {
			index: ALL_IMAGES,
			wimstruct: self.wimstruct,
			_borrows_wim: PhantomData,
		}
	}
}

impl Drop for Wim {
	fn drop(&mut self) {
		unsafe {
			sys::wimlib_free(self.wimstruct);
		}
	}
}

struct SavedProgressCallback(NonNull<*mut ProgressCallback>);
impl Drop for SavedProgressCallback {
	fn drop(&mut self) {
		unsafe {
			let boxed_ptr = Box::from_raw(self.0.as_ptr());
			let _ = Box::from_raw(*boxed_ptr);
		}
	}
}

/// Selected WIM image
///
/// Contains a pointer to underlying [`sys::WIMStruct`], image index and a
/// lifetime marker for its owning value
///
/// This does not ensure that selected image will stay the wanted image. It just
/// stores the index. If you delete this image and other [`Image`]s
/// reference it, operations with it will be invalid. If you the create a new
/// image with same index, it will reference this image.
pub struct Image<'a> {
	wimstruct: *mut sys::WIMStruct,
	index: ImageIndex,
	_borrows_wim: PhantomData<&'a Wim>,
}

impl Image<'_> {
	fn ffi_index(&self) -> i32 {
		self.index.get() as i32
	}
}

/// Specifies a compression type
///
/// A WIM file has a default compression type, indicated by its file header.
/// Normally, each resource in the WIM file is compressed with this compression
/// type. However, resources may be stored as uncompressed; for example, wimlib
/// may do so if a resource does not compress to less than its original size.
/// In addition, a WIM with the new version number of 3584, or "ESD file",
/// might contain solid resources with different compression types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(i32)]
#[non_exhaustive]
pub enum CompressionType {
	/// No compression
	None = sys::wimlib_compression_type_WIMLIB_COMPRESSION_TYPE_NONE as _,
	/// The XPRESS compression format
	///
	/// This format combines Lempel-Ziv factorization with Huffman encoding.
	/// Compression and decompression are both fast. This format supports chunk
	/// sizes that are powers of 2 between 2¹² and 2¹⁶, inclusively.
	///
	/// wimlib's XPRESS compressor will, with the default settings, usually
	/// produce a better compression ratio, and work more quickly, than the
	/// implementation in Microsoft's WIMGAPI (as of Windows 8.1). Non-default
	/// compression levels are also supported. For example, level 80 will enable
	/// two-pass optimal parsing, which is significantly slower but usually
	/// improves compression by several percent over the default level of 50.
	Xpress = sys::wimlib_compression_type_WIMLIB_COMPRESSION_TYPE_XPRESS as _,
	/// The LZX compression format
	///
	/// This format combines Lempel-Ziv factorization with Huffman encoding, but
	/// with more features and complexity than XPRESS. Compression is slow to
	/// somewhat fast, depending on the settings. Decompression is fast but
	/// slower than XPRESS. This format supports chunk sizes that are powers of
	/// 2 between 2¹⁵ and 2²¹, inclusively. Note: Chunk sizes other than 2^¹⁵
	/// are not compatible with the Microsoft implementation
	///
	/// wimlib's LZX compressor will, with the default settings, usually produce
	/// a better compression ratio, and work more quickly, than the
	/// implementation in Microsoft's WIMGAPI (as of Windows 8.1). Non-default
	/// compression levels are also supported. For example, level 20 will
	/// provide fast compression, almost as fast as XPRESS.
	Lzx = sys::wimlib_compression_type_WIMLIB_COMPRESSION_TYPE_LZX as _,
	/// The LZMS compression format
	///
	/// This format combines Lempel-Ziv factorization with adaptive Huffman
	/// encoding and range coding. Compression and decompression are both fairly
	/// slow. This format supports chunk sizes that are powers of 2 between 2¹⁵
	/// and 2³⁰, inclusively. This format is best used for large chunk sizes.
	/// Note: LZMS compression is only compatible with wimlib v1.6.0 and later,
	/// WIMGAPI Windows 8 and later, and DISM Windows 8.1 and later. Also, chunk
	/// sizes larger than 2²⁶ are not compatible with the Microsoft
	/// implementation.
	///
	/// wimlib's LZMS compressor will, with the default settings, usually
	/// produce a better compression ratio, and work more quickly, than the
	/// implementation in Microsoft's WIMGAPI (as of Windows 8.1). There is
	/// limited support for non-default compression levels, but compression will
	/// be noticeably faster if you choose a level < 35.
	Lzms = sys::wimlib_compression_type_WIMLIB_COMPRESSION_TYPE_LZMS as _,

	#[doc(hidden)]
	__Unknown = -1,
}

impl CompressionType {
	/// Create [`CompressionType`] from it's C representation
	pub fn from_raw(val: i32) -> Self {
		match val {
			0 => Self::None,
			1 => Self::Xpress,
			2 => Self::Lzx,
			3 => Self::Lzms,
			_ => Self::__Unknown,
		}
	}
}
