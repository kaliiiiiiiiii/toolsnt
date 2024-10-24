//! Retrieve information about a WIM or WIM image

use {
	super::{CompressionType, Image, ImageIndex, Wim},
	crate::{error::result_from_raw, string::TStr, sys, Error},
	chrono::{DateTime, Utc},
	std::{
		ffi, fmt::Debug, marker::PhantomData, mem::MaybeUninit, num::NonZero, ptr::null_mut, rc::Rc,
	},
	uuid::Uuid,
	widestring::U16Str,
};

impl Wim {
	/// Get basic information about a WIM file
	#[doc(alias = "wimlib_get_wim_info")]
	pub fn info(&self) -> WimInfo {
		let raw_info = unsafe {
			let mut raw_info = MaybeUninit::uninit();
			sys::wimlib_get_wim_info(self.wimstruct, raw_info.as_mut_ptr());
			raw_info.assume_init()
		};

		WimInfo::from_raw(raw_info)
	}

	/// Read a WIM file's XML document into a buffer
	///
	/// The XML document contains metadata about the WIM file and the images
	/// stored in it.
	///
	/// # Error values
	/// - [`Error::NoFilename`]: WIM is not backed by a file and therefore does
	///   not have an XML document
	/// - [`Error::Read`]: Failed to read the XML document from the WIM file
	/// - [`Error::UnexpectedEndOfFile`]: Failed to read the XML document from
	///   the WIM file
	#[doc(alias = "wimlib_get_xml_data")]
	pub fn xml_data(&self) -> Result<&U16Str, Error> {
		let mut out_buf_data = null_mut();
		let mut out_buf_len = 0;

		result_from_raw(unsafe {
			sys::wimlib_get_xml_data(self.wimstruct, &mut out_buf_data, &mut out_buf_len)
		})?;

		let widestr = unsafe { U16Str::from_ptr(out_buf_data.cast(), out_buf_len) };
		Ok(widestr)
	}

	/// Determine if an image name is already used by some image in the WIM
	///
	/// # Note
	/// If `name` is empty, `false` is returned.
	#[doc(alias = "wimlib_image_name_in_use")]
	pub fn image_name_in_use(&self, name: &TStr) -> bool {
		unsafe { sys::wimlib_image_name_in_use(self.wimstruct, name.as_ptr()) }
	}

	/// Iterate through the blob lookup table of a [`Wim`]
	///
	/// This can be used to directly get a listing of the unique »blobs«
	/// contained in a WIM file, which are deduplicated over all images.
	///
	/// Specifically, each listed blob may be from any of the following sources:
	/// - Metadata blobs, if the [`Wim`] contains image metadata
	/// - File blobs from the on-disk WIM file (if any) backing the [`Wim`]
	/// - File blobs from files that have been added to the in-memory [`Wim`],
	///   e.g. by using [`Wim::add_image`]
	/// - File blobs from external WIMs referenced by
	///   [`Wim::reference_resource_files`] or [`Wim::reference_resources`]
	///
	/// # Error values
	/// - Error value returned by the callback
	#[doc(alias = "wimlib_iterate_lookup_table")]
	pub fn iterate_lookup_table(
		&self,
		flags: IterateLookupTableFlags,
		callback: &mut IterateLookupTableCallback,
	) -> Result<(), Error> {
		let callback_thin_ptr: *mut *mut IterateLookupTableCallback =
			&mut (callback as *mut _) as *mut _;

		result_from_raw(unsafe {
			sys::wimlib_iterate_lookup_table(
				self.wimstruct,
				flags.bits(),
				Some(iterate_lookup_table_trampoline),
				callback_thin_ptr.cast(),
			)
		})
	}

	/// Print the header of the WIM file (intended for debugging only)
	#[doc(alias = "wimlib_print_header")]
	pub fn print_header(&self) {
		unsafe { sys::wimlib_print_header(self.wimstruct) }
	}

	/// Translate a string specifying the name or number of an image in the WIM
	/// into the number of the image
	///
	/// The images are numbered starting at 1.
	///
	/// # `image_name_or_num` parameter note
	/// A string specifying the name or number of an
	/// image in the WIM. If it parses to a positive integer, this integer is
	/// taken to specify the number of the image, indexed starting at 1.
	/// Otherwise, it is taken to be the name of an image, as given in the XML
	/// data for the WIM file. It also may be the keyword `"all"` or the
	/// string `"*"`, both of which will resolve to [`super::ALL_IMAGES`].
	///
	/// There is no way to search for an image actually named `"all"`, `"*"`, or
	/// an integer number, or an image that has no name. However, you can use
	/// [`Image::property`] to get the name of any image.
	///
	/// # Return note
	/// If the string resolved to a single existing image, the number of that
	/// image, indexed starting at 1, is returned. If the keyword "all" or "*"
	/// was specified, [`super::ALL_IMAGES`] is returned. Otherwise,
	/// [`None`] is returned. If image_name_or_num was empty string,
	/// [`None`] is returned, even if one or more images in wim has no
	/// name. (Since a WIM may have multiple unnamed images, an unnamed image
	/// must be specified by index to eliminate the ambiguity.)
	#[doc(alias = "wimlib_resolve_image")]
	pub fn resolve_image(&self, image_name_or_num: &TStr) -> Option<ImageIndex> {
		let value =
			unsafe { sys::wimlib_resolve_image(self.wimstruct, image_name_or_num.as_ptr()) };

		NonZero::new(value as u32)
	}
}

impl<'a> Image<'a> {
	/// Get a per-image property from the WIM's XML document
	///
	/// # Property name
	/// The name of the image property, for example `"NAME"`, `"DESCRIPTION"`,
	/// or `"TOTALBYTES"`. The name can contain forward slashes to indicate a
	/// nested XML element; for example, `"WINDOWS/VERSION/BUILD"` indicates the
	/// `BUILD` element nested within the `VERSION` element nested within the
	/// `WINDOWS` element.
	///
	/// Since wimlib v1.9.0, a bracketed number can be used to
	/// indicate one of several identically-named elements; for example,
	/// `"WINDOWS/LANGUAGES/LANGUAGE[2]"` indicates the second `"LANGUAGE"`
	/// element nested within the `"WINDOWS/LANGUAGES"` element. Note that
	/// element names are case sensitive.
	pub fn property(&self, property_name: &TStr) -> Option<&'a TStr> {
		unsafe {
			let ptr = sys::wimlib_get_image_property(
				self.wimstruct,
				self.ffi_index(),
				property_name.as_ptr(),
			);

			TStr::from_ptr_optional(ptr)
		}
	}

	/// Iterate through a file or directory tree in a WIM image
	///
	/// By specifying appropriate flags and a callback function, you can get the
	/// attributes of a file in the image, get a directory listing, or even get
	/// a listing of the entire image.
	///
	/// # Error values
	/// - [`Error::InvalidImage`]: Image does not exist in WIM
	/// - [`Error::PathDoesNotExist`]: Path does not exist in the image
	/// - [`Error::ResourceNotFound`]: Path does not exist in the image
	/// - Additionally [`Error::Decompression`],
	///   [`Error::InvalidMetadataResource`], [`Error::MetadataNotFound`],
	///   [`Error::Read`] or [`Error::UnexpectedEndOfFile`],  all of which
	///   indicate failure (for different reasons) to read the metadata resource
	///   for an image over which iteration needed to be done.
	/// - Error value returned by the callback
	#[doc(alias = "wimlib_iterate_dir_tree")]
	pub fn iterate_dir_tree(
		&self,
		path: &TStr,
		flags: IterateDirTreeFlags,
		callback: &mut IterateDirTreeCallback,
	) -> Result<(), Error> {
		let callback_thin_ptr: *mut *mut IterateDirTreeCallback =
			&mut (callback as *mut _) as *mut _;

		result_from_raw(unsafe {
			sys::wimlib_iterate_dir_tree(
				self.wimstruct,
				self.ffi_index(),
				path.as_ptr(),
				flags.bits(),
				Some(iterate_dir_tree_trampoline),
				callback_thin_ptr.cast(),
			)
		})
	}
}

macro_rules! gen_bitfield_bool_getters {
	{
		field: $field:ident;
		$(
			$(#[$meta:meta])*
			$name:ident : $index:expr
		),* $(,)?
	} => {
		$(
			$(#[$meta])*
			pub fn $name(&self) -> bool {
				self.$field.get_bit($index)
			}
		)*
	};
}

/// General information about a WIM file
///
/// This info can also be requested for a [`Wim`] that does not have a backing
/// file. In this case, fields that only make sense given a backing file are set
/// to default values.
#[doc(alias = "wimlib_wim_info")]
pub struct WimInfo {
	/// The globally unique identifier for this WIM
	pub guid: Uuid,
	/// The number of images in this WIM file
	pub image_count: u32,
	/// The 1-based index of the bootable image in this WIM file, or 0 if no
	/// image is bootable
	pub boot_index: u32,
	/// The version of the WIM file format used in this WIM file
	pub wim_version: u32,
	/// The default compression chunk size of resources in this WIM file
	pub chunk_size: u32,
	/// For split WIMs, the 1-based index of this part within the split WIM;
	/// otherwise 1
	pub part_number: u16,
	/// For split WIMs, the total number of parts in the split WIM; otherwise 1
	pub total_parts: u16,
	/// The default compression type of resources in this WIM file
	pub compression_type: CompressionType,
	/// The size of this WIM file in bytes, excluding the XML data and integrity
	/// table
	pub total_bytes: u64,

	bitfield_attrs: sys::__BindgenBitfieldUnit<[u8; 4]>,
}

impl WimInfo {
	/// Create [`WimInfo`] from C structure
	pub fn from_raw(ffi: sys::wimlib_wim_info) -> Self {
		Self {
			guid: Uuid::from_bytes(ffi.guid),
			image_count: ffi.image_count,
			boot_index: ffi.boot_index,
			wim_version: ffi.wim_version,
			chunk_size: ffi.chunk_size,
			part_number: ffi.part_number,
			total_parts: ffi.total_parts,
			compression_type: CompressionType::from_raw(ffi.compression_type),
			total_bytes: ffi.total_bytes,
			bitfield_attrs: ffi._bitfield_1,
		}
	}

	gen_bitfield_bool_getters! {
		field: bitfield_attrs;

		/// WIM file has an integrity table
		has_integrity_table : 0,
		/// Info struct is for a [`Wim`] that has a backing file
		opened_from_file    : 1,
		/// This WIM file is considered readonly for any reason (e.g. the »readonly« header flag is set, or this is part of a split WIM, or filesystem permissions deny writing)
		is_readonly         : 2,
		/// The »reparse point fix« flag is set in this WIM's header
		has_rpfix           : 3,
		/// The »readonly« flag is set in this WIM's header
		is_marked_readonly  : 4,
		/// The »spanned« flag is set in this WIM's header
		spanned             : 5,
		/// The »write in progress« flag is set in this WIM's header
		write_in_progress   : 6,
		/// The »metadata only« flag is set in this WIM's header
		metadata_only       : 7,
		/// The »resource only« flag is set in this WIM's header
		resource_only       : 8,
		/// This WIM file is pipable (see WIMLIB_WRITE_FLAG_PIPABLE)
		pipable             : 9,
	}
}

/// Information about a "blob", which is a fixed length sequence of binary data
///
/// Each nonempty stream of each file in a WIM image is associated with a blob.
/// Blobs are deduplicated within a WIM file.
///
/// TODO: this struct needs to be renamed, and perhaps made into a enum since
/// there are several cases. I'll try to list them below:
/// 1. The blob is "missing", meaning that it is referenced by hash but not
///    actually present in the WIM file. In this case we only know the
///    sha1_hash. This case can only occur with wimlib_iterate_dir_tree(), never
///    wimlib_iterate_lookup_table().
#[doc(alias = "wimlib_resource_entry")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourceEntry {
	/// If this blob is not missing, then this is the uncompressed size of this
	/// blob in bytes
	pub uncompressed_size: u64,
	/// If this blob is located in a non-solid WIM resource, then this is the
	/// compressed size of that resource
	pub compressed_size: u64,
	/// If this blob is located in a non-solid WIM resource, then this is the
	/// offset of that resource within the WIM file containing it
	pub offset: u64,
	/// If this blob is located in a WIM resource, then this is the SHA-1
	/// message digest of the blob's uncompressed contents
	pub sha1_hash: [u8; 20],
	/// If this blob is located in a WIM resource, then this is the part number
	/// of the WIM file containing it
	pub part_number: u32,
	/// If this blob is not missing, then this is the number of times this blob
	/// is referenced over all images in the WIM
	pub reference_count: u32,

	bitfield_attrs: sys::__BindgenBitfieldUnit<[u8; 4]>,

	/// If this blob is located in a solid WIM resource,
	/// then this is the offset of thatsolid resource within the WIM file
	/// containing it
	pub raw_resource_offset_in_wim: u64,
	/// If this blob is located in a solid WIM resource,
	/// then this is the compressed size of that solid resource
	pub raw_resource_compressed_size: u64,
	/// If this blob is located in a solid WIM resource,
	/// then this is the uncompressed size of that solid resource
	pub raw_resource_uncompressed_size: u64,
}

impl ResourceEntry {
	/// Create [`WimInfo`] from C structure
	pub fn from_raw(ffi: sys::wimlib_resource_entry) -> Self {
		Self {
			uncompressed_size: ffi.uncompressed_size,
			compressed_size: ffi.compressed_size,
			offset: ffi.offset,
			sha1_hash: ffi.sha1_hash,
			part_number: ffi.part_number,
			reference_count: ffi.reference_count,
			bitfield_attrs: ffi._bitfield_1,
			raw_resource_offset_in_wim: ffi.raw_resource_offset_in_wim,
			raw_resource_compressed_size: ffi.raw_resource_compressed_size,
			raw_resource_uncompressed_size: ffi.raw_resource_uncompressed_size,
		}
	}

	gen_bitfield_bool_getters! {
		field: bitfield_attrs;

		/// Blob is located in a non-solid compressed WIM resource
		is_compressed: 0,
		/// Blob contains the metadata for an image
		is_metadata:   1,
		/// Entry is free
		is_free:       2,
		/// Entry is spanned
		is_spanned:    3,
		/// Blob with this hash was not found in the blob lookup table of the [`Wim`]
		is_missing:    4,
		/// Blob is located in a solid resource
		packed:        5,
	}
}

/// Information about a stream of a particular file in the WIM
///
/// Normally, only WIM images captured from NTFS filesystems will have multiple
/// streams per file. In practice, this is a rarely used feature of the
/// filesystem.
///
/// TODO: the library now explicitly tracks stream types, which allows it to
/// have multiple unnamed streams (e.g. both a reparse point stream and unnamed
/// data stream).
///
/// However, this isn't yet exposed by [`Image::iterate_dir_tree`]
#[doc(alias = "wimlib_stream_entry")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StreamEntry<'a> {
	/// Name of the stream, if named
	pub stream_name: Option<&'a TStr>,
	/// Info about this stream's data, such as its hash and size if known
	pub resource: ResourceEntry,
}

impl<'a> StreamEntry<'a> {
	/// Create [`WimInfo`] from C structure
	pub fn from_raw(ffi: sys::wimlib_stream_entry) -> Self {
		let stream_name =
			(!ffi.stream_name.is_null()).then(|| unsafe { TStr::from_ptr(ffi.stream_name) });

		Self {
			stream_name,
			resource: ResourceEntry::from_raw(ffi.resource),
		}
	}
}

/// Unresolved stream entry
///
/// For more information, refer to [`StreamEntry`]
///
/// Can be converted to [`StreamEntry`] using
/// [`LazyStreamEntry::into_stream_entry`].
///
/// This method is also used for [`From`] trait implementation.
#[doc(alias = "wimlib_stream_entry")]
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct LazyStreamEntry<'a> {
	raw: sys::wimlib_stream_entry,
	_lt: PhantomData<StreamEntry<'a>>,
}

impl<'a> LazyStreamEntry<'a> {
	/// Evaluate lazy entry into a completed one
	pub fn into_stream_entry(self) -> StreamEntry<'a> {
		StreamEntry::from_raw(self.raw)
	}
}

impl<'a> Debug for LazyStreamEntry<'a> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("LazyStreamEntry").finish_non_exhaustive()
	}
}

impl<'a> From<LazyStreamEntry<'a>> for StreamEntry<'a> {
	fn from(value: LazyStreamEntry<'a>) -> Self {
		value.into_stream_entry()
	}
}

/// An object ID, which is an extra piece of metadata that may be associated
/// with a file on NTFS filesystems
#[doc(alias = "wimlib_object_id")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ObjectId {
	/// Object GUID
	pub object_id: Uuid,
	/// Origin volume GUID
	pub birth_volume_id: Uuid,
	/// Original object GUID
	pub birth_object_id: Uuid,
	/// Domain GUID
	pub domain_id: Uuid,
}

impl ObjectId {
	/// Create [`ObjectId`] from it's C representation
	pub fn from_raw(ffi: sys::wimlib_object_id) -> Self {
		Self {
			object_id: Uuid::from_bytes(ffi.object_id),
			birth_volume_id: Uuid::from_bytes(ffi.birth_volume_id),
			birth_object_id: Uuid::from_bytes(ffi.birth_object_id),
			domain_id: Uuid::from_bytes(ffi.domain_id),
		}
	}
}

// TODO: Windows-specific entries

/// Structure passed to the [`Image::iterate_dir_tree`]'s callback
/// function
///
/// Roughly, the information about a »file« in the WIM image — but really a
/// directory entry (»dentry«) because hard links are allowed. The
/// `hard_link_group_id` field can be used to distinguish actual file inodes.
#[doc(alias = "wimlib_dir_entry")]
#[derive(Clone, Debug)]
pub struct DirEntry<'a> {
	/// Name of the file, or [`None`] if this file is unnamed
	pub filename: Option<&'a TStr>,
	/// 8.3 name (or »DOS name«, or »short name«) of this file; or [`None`] if
	/// this file has no such name
	pub short_name: Option<&'a TStr>,
	/// Full path to this file within the image
	pub full_path: &'a TStr,
	/// Depth of this directory entry, where 0 is the root, 1 is the root's
	/// children, ..., etc
	pub depth: usize,
	/// File attributes, such as whether the file is a directory or not
	pub file_attributes: FileAttributes,
	/// If the file is a reparse point ([`FileAttributes::REPARSE_POINT`] set in
	/// the attributes), this will give the reparse tag
	pub reparse_tag: ReparseTag,
	/// Number of links to this file's inode (hard links)
	pub num_links: u32,
	/// Number of named data streams this file has
	pub num_named_streams: u32,
	/// A unique identifier for this file's inode
	pub hard_link_group_id: u64,
	/// Time this file was created
	pub creation_time: DateTime<Utc>,
	/// Time this file was last written to
	pub last_write_time: DateTime<Utc>,
	/// Time this file was last accessed
	pub last_access_time: DateTime<Utc>,
	/// The UNIX user ID of this file
	pub unix_uid: u32,
	/// The UNIX group ID of this file
	pub unix_gid: u32,
	/// The UNIX mode of this file
	pub unix_mode: u32,
	/// The UNIX device ID (major and minor number) of this file
	pub unix_rdev: u32,
	/// Object's ID
	pub object_id: ObjectId,
	/// File's stream entries
	pub streams: &'a [LazyStreamEntry<'a>],
}

impl<'a> DirEntry<'a> {
	/// Get evaluated streams
	pub fn streams_converted(&self) -> Rc<[StreamEntry<'a>]> {
		self.streams
			.iter()
			.map(|entry| entry.into_stream_entry())
			.collect()
	}

	/// # Safety
	/// - `ffi` has to be valid pointer to underlying data
	pub unsafe fn from_raw(ffi: *const sys::wimlib_dir_entry) -> Self {
		unsafe {
			let file_attributes =
				FileAttributes::from_bits((*ffi).attributes).expect("Unsupported file attributes");

			let num_named_streams = (*ffi).num_named_streams;
			let streams = {
				let streams_len = num_named_streams as usize + 1;
				let ptr = (*ffi).streams.as_ptr();

				std::slice::from_raw_parts(ptr.cast(), streams_len)
			};

			let creation_time = convert_timedate((*ffi).creation_time, (*ffi).creation_time_high);
			let last_write_time =
				convert_timedate((*ffi).last_write_time, (*ffi).last_write_time_high);

			let last_access_time =
				convert_timedate((*ffi).last_access_time, (*ffi).last_access_time_high);

			Self {
				filename: TStr::from_ptr_optional((*ffi).filename),
				short_name: TStr::from_ptr_optional((*ffi).dos_name),
				full_path: TStr::from_ptr((*ffi).full_path),
				depth: (*ffi).depth,
				file_attributes,
				reparse_tag: std::mem::transmute::<u32, ReparseTag>((*ffi).reparse_tag),
				num_links: (*ffi).num_links,
				num_named_streams,
				hard_link_group_id: (*ffi).hard_link_group_id,
				creation_time,
				last_write_time,
				last_access_time,
				unix_uid: (*ffi).unix_uid,
				unix_gid: (*ffi).unix_gid,
				unix_mode: (*ffi).unix_mode,
				unix_rdev: (*ffi).unix_rdev,
				object_id: ObjectId::from_raw((*ffi).object_id),
				streams,
			}
		}
	}
}

bitflags::bitflags! {
	/// File attributes are metadata values stored by the file system on disk
	/// and are used by the system and are available to developers via various
	/// file I/O APIs. For a list of related APIs and topics, see the
	/// [See also](https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants#see-also)
	/// section.
	#[doc(alias = "WIMLIB_FILE_ATTRIBUTE")]
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub struct FileAttributes: std::ffi::c_uint {
		/// A file that is read-only
		///
		/// Applications can read the file, but cannot write to it or delete it.
		/// This attribute is not honored on directories. For more information,
		/// see You cannot view or change the Read-only or the System attributes
		/// of folders in Windows NT 5.1 and newer.
		const READONLY            = sys::WIMLIB_FILE_ATTRIBUTE_READONLY;
		/// The file or directory is hidden
		///
		/// It is not included in an ordinary directory listing.
		const HIDDEN              = sys::WIMLIB_FILE_ATTRIBUTE_HIDDEN;
		/// A file or directory that the operating system uses a part of, or uses
		/// exclusively
		const SYSTEM              = sys::WIMLIB_FILE_ATTRIBUTE_SYSTEM;
		/// The handle that identifies a directory
		const DIRECTORY           = sys::WIMLIB_FILE_ATTRIBUTE_DIRECTORY;
		/// A file or directory that is an archive file or directory
		///
		/// Applications typically use this attribute to mark files for backup
		/// or removal.
		const ARCHIVE             = sys::WIMLIB_FILE_ATTRIBUTE_ARCHIVE;
		/// This value is reserved for system use
		const DEVICE              = sys::WIMLIB_FILE_ATTRIBUTE_DEVICE;
		/// A file that does not have other attributes set
		///
		/// This attribute is valid only when used alone.
		const NORMAL              = sys::WIMLIB_FILE_ATTRIBUTE_NORMAL;
		/// A file that is being used for temporary storage
		///
		/// File systems avoid writing data back to mass storage if sufficient
		/// cache memory is available, because typically, an application deletes
		/// a temporary file after the handle is closed. In that scenario,
		/// the system can entirely avoid writing the data. Otherwise, the data
		/// is written after the handle is closed.
		const TEMPORARY           = sys::WIMLIB_FILE_ATTRIBUTE_TEMPORARY;
		/// A file that is a sparse file
		const SPARSE_FILE         = sys::WIMLIB_FILE_ATTRIBUTE_SPARSE_FILE;
		/// A file or directory that has an associated reparse point, or a file
		/// that is a symbolic link
		const REPARSE_POINT       = sys::WIMLIB_FILE_ATTRIBUTE_REPARSE_POINT;
		/// A file or directory that is compressed
		///
		/// For a file, all of the data in the file is compressed.
		///
		/// For a directory, compression is the default for newly created files
		/// and subdirectories.
		const COMPRESSED          = sys::WIMLIB_FILE_ATTRIBUTE_COMPRESSED;
		/// The data of a file is not available immediately
		///
		/// This attribute indicates that the file data is physically moved to
		/// offline storage. This attribute is used by Remote Storage, which is
		/// the hierarchical storage management software. Applications should not
		/// arbitrarily change this attribute.
		const OFFLINE             = sys::WIMLIB_FILE_ATTRIBUTE_OFFLINE;
		/// The file or directory is not to be indexed by the content indexing
		/// service
		const NOT_CONTENT_INDEXED = sys::WIMLIB_FILE_ATTRIBUTE_NOT_CONTENT_INDEXED;
		/// A file or directory that is encrypted
		///
		/// For a file, all data streams in the file are encrypted.
		///
		/// For a directory, encryption is the default for newly created files and
		/// subdirectories.
		const ENCRYPTED           = sys::WIMLIB_FILE_ATTRIBUTE_ENCRYPTED;
		/// This value is reserved for system use
		const VIRTUAL             = sys::WIMLIB_FILE_ATTRIBUTE_VIRTUAL;
	}
}

/// A unique identifier for a file system filter driver stored within a file's
/// optional reparse point data that indicates the file system filter driver
/// that performs additional filter-defined processing on a file during I/O
/// operations.
///
/// An implementer can request more than one reparse point for use with a file
/// system, a file system filter driver, or a minifilter driver. To request a
/// reparse point tag, use the reparse point tag request form.
///
/// For more information, see
/// [\[WHDC-RPTR\]](https://learn.microsoft.com/en-us/windows-hardware/drivers/ifs/reparse-point-tag-request)
#[doc(alias = "WIMLIB_REPARSE_TAG")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
#[non_exhaustive]
pub enum ReparseTag {
	/// Reserved reparse tag value
	Reserved0 = sys::WIMLIB_REPARSE_TAG_RESERVED_ZERO,
	/// Reserved reparse tag value
	Reserved1 = sys::WIMLIB_REPARSE_TAG_RESERVED_ONE,
	/// Used for mount point support
	///
	/// Specified in <https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/ca069dad-ed16-42aa-b057-b6b207f447cc>
	MountPoint = sys::WIMLIB_REPARSE_TAG_MOUNT_POINT,
	/// Obsolete. Used by legacy Hierarchical Storage Manager Product
	Hsm = sys::WIMLIB_REPARSE_TAG_HSM,
	/// Obsolete. Used by legacy Hierarchical Storage Manager Product
	Hsm2 = sys::WIMLIB_REPARSE_TAG_HSM2,
	/// Home server drive extender
	DriverExtender = sys::WIMLIB_REPARSE_TAG_DRIVER_EXTENDER,
	/// Used by [single-instance storage (SIS)](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/8ac44452-328c-4d7b-a784-d72afd19bd9f#gt_40c57198-06d7-4dcb-a011-d8152b64d2cb) filter driver
	///
	/// Server-side interpretation only, not meaningful over the wire.
	Sis = sys::WIMLIB_REPARSE_TAG_SIS,
	/// Used by the DFS filter.
	///
	/// The DFS is described in the Distributed File System (DFS): Referral
	/// Protocol Specification [\[MS-DFSC\]](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dfsc/3109f4be-2dbb-42c9-9b8e-0b34f7a2135e).
	/// Server-side interpretation only, not meaningful over the wire.
	Dfs = sys::WIMLIB_REPARSE_TAG_DFS,
	/// Used by the DFS filter.
	///
	/// The DFS is described in the Distributed File System (DFS): Referral
	/// Protocol Specification [\[MS-DFSC\]](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-dfsc/3109f4be-2dbb-42c9-9b8e-0b34f7a2135e).
	/// Server-side interpretation only, not meaningful over the wire.
	Dfsr = sys::WIMLIB_REPARSE_TAG_DFSR,
	/// Used by [filter manager](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/8ac44452-328c-4d7b-a784-d72afd19bd9f#gt_09f024eb-3598-47db-91e0-37c83b82bf68) test harness
	FilterManager = sys::WIMLIB_REPARSE_TAG_FILTER_MANAGER,
	/// Used by the Windows Overlay filter, for either WIMBoot or single-file
	/// compression.
	///
	/// Server-side interpretation only, not meaningful over the wire.
	Wof = sys::WIMLIB_REPARSE_TAG_WOF,
	/// Used for [symbolic link](https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/8ac44452-328c-4d7b-a784-d72afd19bd9f#gt_04f1ed93-15cb-4090-8204-c43bec8c7398)
	/// support.
	///
	/// See <https://learn.microsoft.com/en-us/openspecs/windows_protocols/ms-fscc/b41f1cbf-10df-4a47-98d4-1c52a833d913>
	Symlink = sys::WIMLIB_REPARSE_TAG_SYMLINK,
}

fn convert_timedate(timespec: sys::timespec, _high_secs: i32) -> DateTime<Utc> {
	// TODO: Support 32-bit timestamp
	let secs = timespec.tv_sec;
	let nsecs = timespec.tv_nsec as u32;

	DateTime::from_timestamp(secs, nsecs).expect("Valid time format from FFI")
}

/// Callback for [`Image::iterate_dir_tree`]
#[doc(alias = "wimlib_iterate_dir_tree_callback_t")]
type IterateDirTreeCallback = dyn FnMut(DirEntry) -> Result<(), Error>;

unsafe extern "C" fn iterate_dir_tree_trampoline(
	dentry: *const sys::wimlib_dir_entry,
	user_ctx: *mut ffi::c_void,
) -> ffi::c_int {
	unsafe {
		let dir_entry = DirEntry::from_raw(dentry);
		let callback_ptr = user_ctx as *mut *mut IterateDirTreeCallback;
		let result = (**callback_ptr)(dir_entry);
		match result {
			Ok(()) => 0,
			Err(e) => e as _,
		}
	}
}

bitflags::bitflags! {
	/// Flags for [`Image::iterate_dir_tree`]
	pub struct IterateDirTreeFlags: ffi::c_int {
		/// Iterate recursively on children rather than just on the specified path
		const RECURSIVE        = sys::WIMLIB_ITERATE_DIR_TREE_FLAG_RECURSIVE        as _;
		/// Don't iterate on the file or directory itself; only its children
		/// (in the case of a non-empty directory)
		const CHILDREN         = sys::WIMLIB_ITERATE_DIR_TREE_FLAG_CHILDREN         as _;
		/// Return [`Error::ResourceNotFound`] if any file data blobs needed to
		/// fill in the [`ResourceEntry`]'s for the iteration cannot be found in
		/// the blob lookup table of the [`WIM`]
		const RESOURCES_NEEDED = sys::WIMLIB_ITERATE_DIR_TREE_FLAG_RESOURCES_NEEDED as _;
	}
}

/// Callback for [`Wim::iterate_lookup_table`]
#[doc(alias = "wimlib_iterate_lookup_table_callback_t")]
type IterateLookupTableCallback = dyn FnMut(ResourceEntry) -> Result<(), Error>;

unsafe extern "C" fn iterate_lookup_table_trampoline(
	resource: *const sys::wimlib_resource_entry,
	user_ctx: *mut ffi::c_void,
) -> ffi::c_int {
	unsafe {
		let resource_entry = ResourceEntry::from_raw(*resource);
		let callback_ptr = user_ctx as *mut *mut IterateLookupTableCallback;
		let result = (**callback_ptr)(resource_entry);
		match result {
			Ok(()) => 0,
			Err(e) => e as _,
		}
	}
}

bitflags::bitflags! {
	/// Flags for [`Wim::interate_lookup_table`]; empty, reserved for future use
	pub struct IterateLookupTableFlags: ffi::c_int {}
}
