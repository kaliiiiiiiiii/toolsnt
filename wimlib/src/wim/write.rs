//! Create or update an on-disk WIM file
//!
//! [`Image::write`] creates a new on-disk WIM file, whereas [`Wim::overwrite`]
//! updates an existing WIM file. See [Basic WIM handling
//! concepts](crate#basic-wim-handling-concepts) for more information about the
//! API design.

use {
	crate::{error::result_from_raw, string::TStr, sys, CompressionType, Error, Image, Wim},
	std::{fs::File, os::fd::AsRawFd},
};

impl Wim {
	/// Commit this [`Wim`] to disk, updating its backing file
	///
	/// There are several alternative ways in which changes may be committed:
	/// 1. Full rebuild: write the updated WIM to a temporary file, then rename
	///    the temporary file to the original
	/// 2. Appending: append updates to the new original WIM file, then
	///    overwrite its header such that those changes become visible to new
	///    readers
	/// 3. Compaction: normally should not be used; see
	///    [`WriteFlags::UNSAFE_COMPACT`] for details.
	///
	/// Append mode is often much faster than a full rebuild, but it wastes some
	/// amount of space due to leaving »holes« in the WIM file. Because of the
	/// greater efficiency, [`Self::overwrite`] normally defaults to append
	/// mode. However, [`WriteFlags::REBUILD`] can be used to explicitly
	/// request a full rebuild. In addition, if [`Image::delete_self`] has
	/// been used on the owning [`Wim`], then the default mode switches to
	/// rebuild mode, and [`WriteFlags::SOFT_DELETE`] can be used to explicitly
	/// request append mode.
	///
	/// # Parameters
	/// - `write_flags`: Extra flags
	/// - `num_threads`: The number of threads to use for compressing data, or
	///   use 0 to have the library automatically choose an appropriate number
	///
	/// # Error values
	/// - Refer to [`Image::write`]
	/// - [`Error::AlreadyLocked`]: Another process is currently modifying the
	///   WIM file
	/// - [`Error::NoFilename`]: This WIM is not backed by an on-disk-file. In
	///   other words, [`Wim`] has been created by
	///   [`crate::WimLib::create_new_wim`] rather than
	///   [`crate::WimLib::open_wim`]
	/// - [`Error::Rename`]: The temporary file to which the WIM was written
	///   could not be renamed to the original file
	/// - [`Error::WimIsReadonly`]: The WIM file is considered read-only because
	///   of any of the reasons mentioned in the documentation for the
	///   [`crate::OpenFlags::WRITE_ACCESS`] flag.
	#[doc(alias = "wimlib_overwrite")]
	pub fn overwrite(self, write_flags: WriteFlags, num_threads: u32) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_overwrite(self.wimstruct, write_flags.bits(), num_threads)
		})
	}

	/// Set output compression chunk size
	///
	/// This is the compression chunk size that will be used for writing
	/// non-solid resources in subsequent calls to [`Image::write`] or
	/// [`Self::overwrite`]. A larger compression chunk size often results in a
	/// better compression ratio, but compression may be slower and the speed of
	/// random access to data may be reduced. In addition, some chunk sizes are
	/// not compatible with Microsoft software.
	///
	/// The valid chunk sizes are dependent on the compression type. See the
	/// documentation for each [`CompressionType`] variant for more
	/// information. As a special case, if chunk_size is specified as 0, then
	/// the chunk size will be reset to the default for the currently selected
	/// output compression type.
	///
	/// # Error values
	/// - [`Error::InvalidChunkSize`]: `chunk_size` was not 0 or a supported
	///   chunk size for the currently selected output compression type
	#[doc(alias = "wimlib_set_output_chunk_size")]
	pub fn set_output_chunk_size(&self, chunk_size: u32) -> Result<(), Error> {
		result_from_raw(unsafe { sys::wimlib_set_output_chunk_size(self.wimstruct, chunk_size) })
	}

	/// Similar to [`Self::set_output_chunk_size`], but set the chunk size for
	/// writing solid resources.)
	///
	/// # Error values
	/// Refer to [`Self::set_output_chunk_size`]
	#[doc(alias = "wimlib_set_output_pack_chunk_size")]
	pub fn set_output_pack_chunk_size(&self, chunk_size: u32) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_set_output_pack_chunk_size(self.wimstruct, chunk_size)
		})
	}

	/// Set output compression type
	///
	/// This is the compression type that will be used for writing non-solid
	/// resources in subsequent calls to [`Image::write`] or
	/// [`Self::overwrite`].
	///
	/// # Error values
	/// - [`Error::InvalidCompressionType`]: `compression_type` did not specify
	///   a valid compression type
	#[doc(alias = "wimlib_set_output_compression_type")]
	pub fn set_output_compression_type(
		&self,
		compression_type: CompressionType,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_set_output_compression_type(self.wimstruct, compression_type as _)
		})
	}

	/// Similar to [`Self::set_output_compression_type`], but set the
	/// compression type for writing solid resources
	///
	/// This cannot be [`CompressionType::None`].
	///
	/// # Error values
	/// Refer to [`Self::set_output_compression_type`]
	#[doc(alias = "wimlib_set_output_pack_compression_type")]
	pub fn set_output_pack_compression_type(
		&self,
		compression_type: CompressionType,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_set_output_compression_type(self.wimstruct, compression_type as _)
		})
	}
}

impl<'a> Image<'a> {
	/// Persist a [`Wim`] to a new on-disk WIM file
	///
	/// This brings in file data from any external locations, such as directory
	/// trees or NTFS volumes scanned with [`Wim::add_image`], or other WIM
	/// files via [`Image::export`], and incorporates it into a new
	/// on-disk WIM file.
	///
	/// By default, the new WIM file is written as stand-alone. Using the
	/// [`WriteFlags::SKIP_EXTERNAL_WIMS`] flag, a »delta« WIM can be written
	/// instead. However, this function cannot directly write a »split« WIM; use
	/// [`Wim::split`] for that.
	///
	/// # Parameters
	/// - `path`: The path to the on-disk file to write
	/// - `write_flags`: Extra flags
	/// - `num_threads`: The number of threads to use for compressing data, or
	///   use 0 to have the library automatically choose an appropriate number
	///
	/// # Error values
	/// - [`Error::ConcurrentModificationDetected`]: A file that had previously
	///   been scanned for inclusion in the WIM was concurrently modified
	/// - [`Error::InvalidImage`]: This image does not exist in owning WIM
	/// - [`Error::InvalidResourceHash`]: A file, stored in another WIM, which
	///   needed to be written was corrupt
	/// - [`Error::InvalidParam`]: `path` was not an empty string, or invalid
	///   flags were passed
	/// - [`Error::Open`]: Failed to open the output WIM file for writing, or
	///   failed to open a file whose data needed to be included in the WIM
	/// - [`Error::Read`]: Failed to read data that needed to be included in the
	///   WIM
	/// - [`Error::ResourceNotFound`]: A file data blob that needed to be
	///   written could not be found in the blob lookup table of WIM. See
	///   [Creating and handling non-standalone
	///   WIMs](crate::wim::non_standalone).
	/// - [`Error::Write`]: An error occurred when trying to write data to the
	///   new WIM file
	///
	/// This function can additionally return [`Error::Decompression`],
	/// [`Error::InvalidMetadataResource`], [`Error::MetadataNotFound`],
	/// [`Error::Read`], or [`Error::UnexpectedEndOfFile`], all of which
	/// indicate failure (for different reasons) to read the data from a WIM
	/// file.
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::WriteStreams`]
	/// - [`crate::progress::ProgressMsg::WriteMetadataBegin`]
	/// - [`crate::progress::ProgressMsg::WriteMetadataEnd`]
	pub fn write(
		&self,
		path: &TStr,
		write_flags: WriteFlags,
		num_threads: u32,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_write(
				self.wimstruct,
				path.as_ptr(),
				self.ffi_index(),
				write_flags.bits(),
				num_threads,
			)
		})
	}

	/// Same as [`Self::write`],  but write the WIM directly to a file
	/// descriptor, which need not be seekable if the write is done in a special
	/// pipable WIM format by providing [`WriteFlags::PIPABLE`] in `write_flags`
	///
	/// This can, for example, allow capturing a WIM image and streaming it over
	/// the network. See [Pipable WIMs](crate#pipable-wims) for more information
	/// about pipable WIMs.
	///
	/// # Error values
	/// - [`Error::InvalidParam`]: File was not seekable, but
	///   [`WriteFlags::PIPABLE`] was not specified in `write_flags`
	pub fn write_to_file(
		&self,
		file: &File,
		write_flags: WriteFlags,
		num_threads: u32,
	) -> Result<(), Error> {
		let raw_handle = file.as_raw_fd();

		result_from_raw(unsafe {
			sys::wimlib_write_to_fd(
				self.wimstruct,
				raw_handle,
				self.ffi_index(),
				write_flags.bits(),
				num_threads,
			)
		})
	}
}

bitflags::bitflags! {
	/// Flags for writing image
	pub struct WriteFlags: std::ffi::c_int {
		/// Include an integrity table in the resulting WIM file
		///
		/// For [`Wim`] created with [`WimLib::open_wim`], the default behavior
		/// is to include an integrity table if and only if one was present
		/// before. For [`Wim`] created with [`WimLib::create_new_wim`], the
		/// default behavior is to not include an integrity table.
		const CHECK_INTEGRITY    = sys::WIMLIB_WRITE_FLAG_CHECK_INTEGRITY       as _;

		/// Do not include an integrity table in the resulting WIM file
		///
		/// This is the default behavior, unless the [`Wim`] was created by
		/// opening a WIM with an integrity table.
		const NO_CHECK_INTEGRITY = sys::WIMLIB_WRITE_FLAG_NO_CHECK_INTEGRITY    as _;

		/// Write the WIM as »pipable«
		///
		/// After writing a WIM with this flag specified, images from it can be
		/// applied directly from a pipe using [`WimLib::extract_image_from_pipe`].
		///
		/// See the documentation for the `–pipable` option of `wimcapture` for
		/// more information.
		///
		/// # Beware
		/// WIMs written with this flag will not be compatible with Microsoft's
		/// software.
		const PIPABLE            = sys::WIMLIB_WRITE_FLAG_PIPABLE               as _;

		/// Do not write the WIM as »pipable«
		///
		/// This is the default behavior, unless the [`Wim`] was created by
		/// opening a pipable WIM.
		const NOT_PIPABLE         = sys::WIMLIB_WRITE_FLAG_NOT_PIPABLE          as _;

		/// When writing data to the WIM file, recompress it, even if the data
		/// is already available in the desired compressed form (for example,
		/// in a WIM file from which an image has been exported using
		/// [`Image::export`])
		///
		/// This flag can be used to recompress with a higher compression ratio
		/// for the same compression type and chunk size. Simply using the
		/// default compression settings may suffice for this, especially if the
		/// IM file was created using another program/library that may not use
		/// as sophisticated compression algorithms. Or,
		/// [`WimLib::set_default_compression_level`] can be called beforehand to
		/// set an even higher compression level than the default.
		///
		/// If the WIM contains solid resources, then this flag can be used in
		/// combination with [`Self::SOLID`] to prevent any solid resources from
		/// being res-used. Otherwise, solid resources are re-used somewhat more
		/// liberally than normal compressed resources.
		///
		/// This flag does **not** cause recompression of data that would not
		/// otherwise be written. For example, a call to [`Wim::overwrite`]
		/// with this flag will not, by itself, cause already-existing data in
		/// the WIM file to be recompressed. To force the WIM file to be fully
		/// rebuilt and recompressed, combine [`Self::RECOMPRESS`] with
		/// [`Self::REBUILD`].
		const RECOMPRESS           = sys::WIMLIB_WRITE_FLAG_RECOMPRESS          as _;

		/// Immediately before closing the WIM file, sync its data to disk
		///
		/// This flag forces the function to wait until the data is safely on
		/// disk before returning success. Otherwise, modern operating systems
		/// tend to cache data for some time (in some cases, 30+ seconds)
		/// before actually writing it to disk, even after reporting to the
		/// application that the writes have succeeded.
		///
		/// [`Wim::overwrite`] will set this flag automatically if it decides
		/// to overwrite the WIM file via a temporary file instead of in-place.
		/// This is necessary on POSIX systems; it will, for example, avoid
		/// problems with delayed allocation on ext4.
		const FSYNC                = sys::WIMLIB_WRITE_FLAG_FSYNC               as _;

		/// For [`Wim::overwrite`]: rebuild the entire WIM file, even if it
		/// otherwise could be updated in-place by appending to it.
		///
		/// Any data that existed in the original WIM file but is not actually
		/// needed by any of the remaining images will not be included. This
		/// can free up space left over after previous in-place modifications
		/// to the WIM file
		///
		/// This flag can be combined with [`Self::RECOMPRESS`] to force all
		/// data to be recompressed. Otherwise, compressed data is re-used if
		/// possible.
		///
		/// [`Image::write`] ignores this flag
		const REBUILD              = sys::WIMLIB_WRITE_FLAG_REBUILD             as _;

		/// For [`Wim::overwrite`]: override the default behavior after one or
		/// more calls to [`Image::delete`], which is to rebuild the
		/// entire WIM file
		///
		/// With this flag, only minimal changes to correctly remove the image
		/// from the WIM file will be taken. This can be much faster, but it
		/// will result in the WIM file getting larger rather than smaller.
		///
		/// [`Image::write`] ignores this flag
		const SOFT_DELETE          = sys::WIMLIB_WRITE_FLAG_SOFT_DELETE          as _;

		/// For [`Wim::overwrite`]: allow overwriting the WIM file even if the
		/// readonly flag (`WIM_HDR_FLAG_READONLY`) is set in the WIM header
		///
		/// This can be used following a call to [`Wim::set_info`] with the
		/// [`crate::SetInfo::readonly_flag`] to actually set the readonly
		/// flag on the on-disk WIM file.
		///
		/// [`Image::write`] ignores this flag
		const IGNORE_READONLY_FLAG = sys::WIMLIB_WRITE_FLAG_IGNORE_READONLY_FLAG as _;

		/// Do not include file data already present in other WIMs
		///
		/// This flag can be used to write a »delta« WIM after the WIM files on
		/// which the delta is to be based were referenced with
		/// [`Wim::reference_resource_files`] or [`Wim::reference_resources`].
		const SKIP_EXTERNAL_WIMS   = sys::WIMLIB_WRITE_FLAG_SKIP_EXTERNAL_WIMS   as _;

		/// For [`Image::write`]: retain the WIM's GUID instead of generating a
		/// new one
		///
		/// [`Wim::overwrite`] sets this by default, since the WIM remains,
		/// logically, the same file.
		const RETAIN_GUID          = sys::WIMLIB_WRITE_FLAG_RETAIN_GUID          as _;

		/// Concatenate files and compress them together, rather than compress
		/// each file independently
		///
		/// This is also known as creating a »solid archive«. This tends to
		/// produce a better compression ratio at the cost of much slower random
		/// access.
		///
		/// WIM files created with this flag are only compatible with wimlib v1.6.0
		/// or later, WIMGAPI Windows 8 or later, and DISM Windows 8.1 or later.
		/// WIM files created with this flag use a different version number in
		/// their header (3584 instead of 68864) and are also called »ESD files«.
		///
		/// Note that providing this flag does not affect the »append by default«
		/// behavior of [`Wim::overwrite`]. In other words, [`Wim::overwrite`] with
		/// just [`Self::SOLID`] can be used to append solid-compressed data to a
		/// WIM file that originally did not contain any solid-compressed data.
		/// But if you instead want to rebuild and recompress an entire WIM file
		/// in solid mode, then also provide [`Self::REBUILD`] and
		/// [`Self::RECOMPRESS`].
		///
		/// Currently, new solid resources will, by default, be written using LZMS
		/// compression with 64 MiB (67108864 byte) chunks.
		/// Use [`Wim::set_output_pack_compression_type`] and/or
		/// [`Wim::set_output_pack_chunk_size`] to change this. This is independent
		/// of the WIM's main compression type and chunk size; you can have a WIM
		/// that nominally uses LZX compression and 32768 byte chunks but actually
		/// contains LZMS-compressed solid resources, for example. However, if
		/// including solid resources, I suggest that you set the WIM's main
		/// compression type to LZMS as well, either by creating the WIM with
		/// specifying compression type or setting it with [`Wim::set_output_compression_type`].
		///
		/// This flag will be set by default when writing or overwriting a WIM
		/// file that either already contains solid resources, or has had solid
		/// resources exported into it and the WIM's main compression type is LZMS.
		const SOLID                 = sys::WIMLIB_WRITE_FLAG_SOLID               as _;

		/// Send [`crate::progress::ProgressMsg::DoneWithFile`] messages when
		/// writing the WIM file.
		///
		/// This is only needed in the unusual case that the library user needs
		/// to know exactly when wimlib has read each file for the last time.
		const SEND_DONE_WITH_FILE_MESSAGES
			= sys::WIMLIB_WRITE_FLAG_SEND_DONE_WITH_FILE_MESSAGES as _;

		/// Do not consider content similarity when arranging file data for
		/// solid compression.
		///
		/// Providing this flag will typically worsen the compression ratio,
		/// so only provide this flag if you know what you are doing.
		const NO_SOLID_SORT         = sys::WIMLIB_WRITE_FLAG_NO_SOLID_SORT       as _;

		/// [`Wim::overwrite`] only: **unsafely** compact the WIM file in-place,
		/// without appending.
		///
		/// Existing resources are shifted down to fill holes and new resources
		/// are appended as needed. The WIM file is truncated to its final size,
		/// which may shrink the on-disk file.
		///
		/// **This operation cannot be safely interrupted. If the operation is
		/// interrupted, then the WIM file will be corrupted, and it may be
		/// impossible (or at least very difficult) to recover any data from it.
		/// Users of this flag are expected to know what they are doing and
		/// assume responsibility for any data corruption that may result.**
		///
		/// If the WIM file cannot be compacted in-place because of its structure,
		/// its layout, or other requested write parameters, then [`Wim::overwrite`]
		/// fails with [`Error::NotPossible`], and the caller may wish to retry
		/// the operation without this flag.
		const UNSAFE_COMPACT        = sys::WIMLIB_WRITE_FLAG_UNSAFE_COMPACT      as _;
	}
}
