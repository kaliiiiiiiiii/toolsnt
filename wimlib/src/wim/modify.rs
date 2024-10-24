//! Make changes to a [`Wim`], in preparation of persisting the [`Wim`] to an
//! on-disk file.
//!
//! # Capturing and adding WIM images
//! As described in [Basic WIM handling
//! concepts](crate#basic-wim-handling-concepts), capturing a new WIM or
//! appending an image to an existing WIM is a multi-step process, but at its
//! core is [`Wim::add_image`] or an equivalent function. Normally,
//! [`Wim::add_image`] takes an on-disk directory tree and logically adds it to
//! a [`Wim`] as a new image. However, when supported by the build of the
//! library, there is also a special NTFS volume capture mode (entered when
//! [`AddFlags::NTFS`] is specified) that allows adding the image directly
//! from an unmounted NTFS volume.
//!
//! Another function, [`Wim::add_image_multisource`] is also provided. It
//! generalizes [`Wim::add_image`] to allow combining multiple files or
//! directory trees into a single WIM image in a configurable way.
//!
//! For maximum customization of WIM image creation, it is also possible to add
//! a completely empty WIM image with [`Wim::add_empty_image`], then update it
//! with [`Image::update`]. (This is in fact what [`Wim::add_image`] and
//! [`Wim::add_image_multisource`] do internally.)
//!
//! Note that some details of how image addition/capture works are documented
//! more fully in the documentation for **wimcapture**.
//!
//! # Deleting WIM images
//! [`Image::delete_self`] can delete an image from a [`Wim`]. But as
//! usual, [`Image::write`] or [`Wim::overwrite`] must be called to
//! cause the changes to be made persistent in an on-disk WIM file.
//!
//! # Exporting WIM images
//! [`Image::export`] can copy, or »export«, an image from one WIM to
//! another.
//!
//! # Other modifications
//! [`Image::update`] can add, delete and rename files in a WIM image. There are
//! also convenience methods using this: [`Image::add_tree`] and
//! [`Image::delete_path`]
//!
//! [`Image::set_property`] can change other image metadata.
//!
//! [`Wim::set_info`] can change information about the WIM file itself, such as
//! the boot index.

use {
	super::{Image, ImageIndex, Wim},
	crate::{
		error::result_from_raw,
		string::{OptionalAsFfiExt, TStr},
		sys,
		wim::new_image_index,
		Error,
	},
	std::{fmt::Debug, marker::PhantomData, mem::MaybeUninit, num::NonZero, ptr::null},
	uuid::Uuid,
};

impl Wim {
	/// Append an empty image to a [`Wim`]
	///
	/// The new image will initially contain no files or directories, although
	/// if written without further modifications, then a root directory will be
	/// created automatically for it.
	///
	/// After calling this function, you can use [`Image::update`] to add
	/// files to the new image. This gives you more control over making the new
	/// image compared to calling [`Wim::add_image`] or
	/// [`Wim::add_image_multisource`]
	///
	/// # Error values
	/// - [`Error::ImageNameCollision`]: The WIM already contains an image with
	///   the requested name
	#[doc(alias = "wimlib_add_empty_image")]
	pub fn add_empty_image(&self, name: &TStr) -> Result<ImageIndex, Error> {
		let mut out_index = 0;

		result_from_raw(unsafe {
			sys::wimlib_add_empty_image(self.wimstruct, name.as_ptr(), &mut out_index)
		})?;

		let index = new_image_index(out_index).expect("FFI returned no image");
		Ok(index)
	}

	/// Add an image to a [`Wim`] from an on-disk directory tree or NTFS volume
	///
	/// The directory tree or NTFS volume is scanned immediately to load the
	/// dentry tree into memory, and file metadata is read. However, actual file
	/// data may not be read until the [`Wim`] is persisted to disk using
	/// [`Image::write`] or [`Self::overwrite`].
	///
	/// See the documentation for the wimlib-imagex program for more information
	/// about the »normal« capture mode versus the NTFS capture mode (entered by
	/// providing the flag [`AddFlags::NTFS`]).
	///
	/// # Parameters
	/// - `source`: A path to a directory or unmounted NTFS volume that will be
	///   captured as a WIM image
	/// - `name`: Name to give the new image. If empty, the new image is given
	///   no name. If nonempty, it must specify a name that does not already
	///   exist in wim.
	/// - `config_file`: Path to the capture configuration file. This file may
	///   specify, among other things, which files to exclude from capture. See
	///   the documentation for `wimcapture` (`–config` option) for details of
	///   the file format. If [`None`], the default capture configuration will
	///   be used. Ordinarily, the default capture configuration will result in
	///   no files being excluded from capture purely based on name; however,
	///   the [`AddFlags::WINCONFIG`] and [`AddFlags::WIMBOOT`] flags modify the
	///   default.
	/// - `add_flags`: Bitflag options
	///
	/// # Error values
	/// This function is implemented by calling [`Self::add_empty_image`], then
	/// calling [`Image::update`] with a single »add« command, so
	/// any error code returned by wimlib_add_empty_image() may be returned, as
	/// well as any error codes returned by [`Image::update`]
	/// other than ones documented as only being returned specifically by an
	/// update involving delete or rename commands.
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::ScanBegin`]
	/// - [`crate::progress::ProgressMsg::ScanEnd`]
	/// - [`crate::progress::ProgressMsg::ScanDentry`] if [`AddFlags::VERBOSE`]
	///   specified
	#[doc(alias = "wimlib_add_image")]
	pub fn add_image(
		&self,
		source: &TStr,
		name: &TStr,
		config_file: Option<&TStr>,
		add_flags: AddFlags,
	) -> Result<(), Error> {
		let config_file_ptr = config_file.map(TStr::as_ptr).unwrap_or(null());

		result_from_raw(unsafe {
			sys::wimlib_add_image(
				self.wimstruct,
				source.as_ptr(),
				name.as_ptr(),
				config_file_ptr,
				add_flags.bits(),
			)
		})
	}

	/// This function is equivalent to [`Self::add_image`] except it allows for
	/// multiple sources to be combined into a single WIM image
	///
	/// See the documentation for `wimcapture` for full details on how this mode
	/// works.
	///
	/// # Parameters
	/// - `source_target_pairs`: Slice of pairs (filesystem_source_path,
	///   wim_target_path)
	/// - `name`, `config_file`, `add_flags`: See [`Self::add_image`]
	#[doc(alias = "wimlib_add_image_multisource")]
	pub fn add_image_multisource(
		&self,
		source_target_pairs: &[SourceTargetPair],
		name: &TStr,
		config_file: Option<&TStr>,
		add_flags: AddFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_add_image_multisource(
				self.wimstruct,
				source_target_pairs.as_ptr().cast(),
				source_target_pairs.len(),
				name.as_ptr(),
				config_file.as_nullable_ptr(),
				add_flags.bits(),
			)
		})
	}

	/// Set basic information about a WIM
	///
	/// # Error values
	/// - [`Error::ImageCount`]: [`SetInfo::boot_index`] was specified, but
	///   [`SetInfo::boot_index`] did not specify [`None`] or a valid `1`-based
	///   image index in the WIM when
	pub fn set_info(&self, info: SetInfo) -> Result<(), Error> {
		// SAFETY: All data inside can be zeroed
		let mut wim_info: sys::wimlib_wim_info = unsafe { MaybeUninit::zeroed().assume_init() };
		let mut which = 0;

		if let Some(flag) = info.readonly_flag {
			wim_info.set_is_marked_readonly(flag as _);
			which |= sys::WIMLIB_CHANGE_READONLY_FLAG;
		}

		if let Some(guid) = info.guid {
			wim_info.guid = guid.into_bytes();
			which |= sys::WIMLIB_CHANGE_GUID;
		}

		if let Some(boot_index) = info.boot_index {
			let boot_index = boot_index.map(NonZero::get).unwrap_or_default();
			wim_info.boot_index = boot_index;
			which |= sys::WIMLIB_CHANGE_BOOT_INDEX;
		}

		if let Some(rpfix_flag) = info.rpfix_flag {
			wim_info.set_has_rpfix(rpfix_flag as _);
			which |= sys::WIMLIB_CHANGE_RPFIX_FLAG;
		}

		result_from_raw(unsafe {
			sys::wimlib_set_wim_info(self.wimstruct, &wim_info, which as i32)
		})
	}
}

/// Pair of source and target for [`Wim::add_image_multisource`]
#[repr(transparent)]
pub struct SourceTargetPair<'a> {
	inner: sys::wimlib_capture_source,
	_borrow: PhantomData<(&'a TStr, &'a TStr)>,
}

impl<'a> SourceTargetPair<'a> {
	/// Create a new wrapped pair of source and target
	pub fn new(source: &'a TStr, target: &'a TStr) -> Self {
		let inner = sys::wimlib_capture_source {
			fs_source_path: source.as_ptr().cast_mut(),
			wim_target_path: target.as_ptr().cast_mut(),
			reserved: 0,
		};

		Self {
			inner,
			_borrow: PhantomData,
		}
	}

	/// Make an array of pairs of source-target
	pub fn array<const N: usize>(pairs: [(&'a TStr, &'a TStr); N]) -> [Self; N] {
		pairs.map(|(src, target)| SourceTargetPair::new(src, target))
	}
}

impl Image<'_> {
	/// Add the file or directory tree at `fs_source_path` on the filesystem to
	/// the location `wim_target_path` within the specified image of the wim
	///
	/// # Notes
	/// Convenience method. Uses [`Self::update`].
	///
	/// Refer to [`UpdateCommand::add`]
	///
	/// # Error values
	/// Refer to [`Self::update`]
	#[doc(alias = "wimlib_add_tree")]
	pub fn add_tree(
		&self,
		fs_source_path: &TStr,
		wim_target_path: &TStr,
		add_flags: AddFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_add_tree(
				self.wimstruct,
				self.ffi_index(),
				fs_source_path.as_ptr(),
				wim_target_path.as_ptr(),
				add_flags.bits(),
			)
		})
	}

	/// Delete image(s) from WIM
	///
	/// # Notes
	/// No changes are commited to disk until [`Image::write`] or
	/// [`Wim::overwrite`] from owning [`Wim`] is called.
	///
	/// This type refers a owning WIM and contains an image index. If image
	/// deleted, other [`Image`]s remain referencing the same image and
	/// will fail if deleted, unless an image with same index is created, then
	/// they will perform operations on it.
	///
	/// # Error values
	/// - [`Error::InvalidImage`]: Image deos not exist in the WIM
	///
	/// ## Metadata read-related
	/// - [`Error::Decompression`]
	/// - [`Error::InvalidMetadataResource`]
	/// - [`Error::MetadataNotFound`]
	/// - [`Error::Read`]
	/// - [`Error::UnexpectedEndOfFile`]
	#[doc(alias = "wimlib_delete_image")]
	pub fn delete_self(self) -> Result<(), Error> {
		result_from_raw(unsafe { sys::wimlib_delete_image(self.wimstruct, self.ffi_index()) })
	}

	/// Delete the `path` from the image
	///
	/// # Notes
	/// Convenience method. Uses [`Self::update`].
	///
	/// Refer to [`UpdateCommand::delete`]
	///
	/// # Error values
	/// Refer to [`Self::update`]
	#[doc(alias = "wimlib_delete_path")]
	pub fn delete_path(&self, path: &TStr, delete_flags: DeleteFlags) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_delete_path(
				self.wimstruct,
				self.ffi_index(),
				path.as_ptr(),
				delete_flags.bits(),
			)
		})
	}

	/// Export image into another [`Wim`]
	///
	/// Specifically, if the destination [`Wim`] contains `n` images, then the
	/// source image will be appended, in order, starting at destination index
	/// `n + 1`.
	///
	/// By default, all image metadata will be exported verbatim, but certain
	/// changes can be made by passing appropriate parameters.
	///
	/// # Notes
	/// - This operation is an in-memory operation; no changes are commited to
	///   disk until [`Image::write`] or [`Wim::overwrite`] is called.
	///
	/// - A limitation of current implementation is that the directory tree of
	///   as source or destination image cannot be updated following and
	///   export until one of two images has been freed from memory.
	///
	/// # Error values
	/// - [`Error::DuplicateExportedImage`]: One or more of the source images
	///   had already been exported into the destination WIM
	/// - [`Error::ImageNameCollision`]: One or more of the names being given to
	///   an exported image was already in use in the destination WIM
	/// - [`Error::InvalidImage`]: The source image doesn't exist
	/// - [`Error::MetadataNotFound`]: At least source or destination WIM
	///   doesn't contain image metadata; for example, one of them represents a
	///   non-first part of a split WIM
	/// - [`Error::ResourceNotFound`]: A file data blob that needed to be
	///   exported could not be found in the blob lookup table of the source
	///   WIM. See [Creating and handling non-standalone
	///   WIMs](crate::wim::non_standalone).
	#[doc(alias = "wimlib_export_image")]
	pub fn export(
		&self,
		dest_wim: &Wim,
		dest_new_name: Option<&TStr>,
		dest_new_description: Option<&TStr>,
		export_flags: ExportFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_export_image(
				self.wimstruct,
				self.ffi_index(),
				dest_wim.wimstruct,
				dest_new_name.as_nullable_ptr(),
				dest_new_description.as_nullable_ptr(),
				export_flags.bits(),
			)
		})
	}

	/// Declare that a newly added image is mostly the same as a prior image,
	/// but captured at a later point in time, possibly with some modifications
	/// in the intervening time
	///
	/// This is designed to be used in incremental backups of the same
	/// filesystem or directory tree.
	///
	/// This function compares the metadata of the directory tree of the newly
	/// added image against that of the old image. Any files that are present in
	/// both the newly added image and the old image and have timestamps that
	/// indicate they haven't been modified are deemed not to have been modified
	/// and have their checksums copied from the old image. Because of this and
	/// because WIM uses single-instance streams, such files need not be read
	/// from the filesystem when the WIM is being written or overwritten. Note
	/// that these unchanged files will still be "archived" and will be
	/// logically present in the new image; the optimization is that they don't
	/// need to actually be read from the filesystem because the WIM already
	/// contains them.
	///
	/// This function is provided to optimize incremental backups. The resulting
	/// WIM file will still be the same regardless of whether this function is
	/// called. (This is, however, assuming that timestamps have not been
	/// manipulated or unmaintained as to trick this function into thinking a
	/// file has not been modified when really it has. To partly guard against
	/// such cases, other metadata such as file sizes will be checked as well.)
	///
	/// # Note
	/// This functions must be called before writing the updated WIM file
	///
	/// # Error values
	/// - [`Error::InvalidImage`]: This image or the template image does not
	///   exist
	/// - [`Error::MetadataNotFound`]: At least this image's or template image's
	///   owning WIM does not contain image matadata; for example, one of them
	///   represents a non-first part of a split WIM
	/// - [`Error::InvalidParam`]: Identical values were provided for the
	///   template and new image; or this image had not been modified since
	///   opening the WIM
	///
	/// ## Metadata read-related
	/// - [`Error::Decompression`]
	/// - [`Error::InvalidMetadataResource`]
	/// - [`Error::MetadataNotFound`]
	/// - [`Error::Read`]
	/// - [`Error::UnexpectedEndOfFile`]
	#[doc(alias = "wimlib_reference_template_image")]
	pub fn reference_template_image(
		&self,
		template_image: &Image,
		flags: ReferenceTemplateImageFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_reference_template_image(
				self.wimstruct,
				self.ffi_index(),
				template_image.wimstruct,
				template_image.ffi_index(),
				flags.bits(),
			)
		})
	}

	/// Rename path of an item
	///
	/// # Notes
	/// Convenience method. Uses [`Self::update`].
	///
	/// Refer to [`UpdateCommand::rename`]
	///
	/// # Error values
	/// Refer to [`Self::update`]
	#[doc(alias = "wimlib_rename_path")]
	pub fn rename_path(&self, from: &TStr, to: &TStr) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_rename_path(self.wimstruct, self.ffi_index(), from.as_ptr(), to.as_ptr())
		})
	}

	/// Add, modify, or remove a per-image property from the WIM's XML document
	///
	/// # Property name
	/// Refer to [`Self::property`]
	///
	/// If [`None`] or empty, the property will be removed
	///
	/// # Error values
	/// - [`Error::ImageNameCollision`]: The user requested to set the image
	///   name (the NAME property), but another image in the WIM already had the
	///   requested name
	/// - [`Error::InvalidImage`]: This image doesn't exist in the WIM
	/// - [`Error::InvalidParam`]: `property_name` has an unsupported format or
	///   it included a bracketed index that was too high
	#[doc(alias = "wimlib_set_image_property")]
	pub fn set_property(&self, name: &TStr, value: Option<&TStr>) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_set_image_property(
				self.wimstruct,
				self.ffi_index(),
				name.as_ptr(),
				value.as_nullable_ptr(),
			)
		})
	}

	// TODO: Set basic information about WIM, `wim_set_wim_info`

	/// Update a WIM image by adding, deleting and/or renaming files or
	/// directories
	///
	/// On failure, all update commands will be rolled back, and no visible
	/// changes will have been made to WIM.
	///
	/// # Error values
	/// - [`Error::FveLockedVolume`]: Windows-only: One of the `add`` commands
	///   attempted to add files from an encrypted BitLocker volume that hasn't
	///   yet been unlocked
	/// - [`Error::ImageHasMultipleReferences`]: There are currently multiple
	///   references to the image as a result of a call to [`Self::export`].
	///   Drop one before attempting the update.
	/// - [`Error::InvalidCaptureConfig`]: The contents of a capture
	///   configuration file were invalid
	/// - [`Error::InvalidImage`]: This image doesn't exist in the WIM
	/// - [`Error::InvalidOverlay`]: An add command with
	///   [`AddFlags::NO_REPLACE`]specified attempted to replace an existing
	///   nondirectory file
	/// - [`Error::InvalidParam`]: An unknown operation type was provided in the
	///   update commands; or unknown or incompatible flags were provided in a
	///   flags parameter; or there was another problem with the provided
	///   parameters
	/// - [`Error::InvalidReparseData`]: While executing an add command, a
	///   reparse point had invalid data
	/// - [`Error::IsDirectory`]: An add command attempted to replace a
	///   directory with a non-directory; or a delete command without
	///   [`DeleteFlags::RECURSIVE`] attempted to delete a directory; or a
	///   rename command attempted to rename a directory to a non-directory
	/// - [`Error::Notdir`]: An add command attempted to replace a non-directory
	///   with a directory; or an add command attempted to set the root of the
	///   image to a non-directory; or a rename command attempted to rename a
	///   directory to a non-directory; or a component of an image path that was
	///   used as a directory was not, in fact, a directory
	/// - [`Error::Notempty`]: A rename command attempted to rename a directory
	///   to a non-empty directory; or a rename command would have created a
	///   loop
	/// - [`Error::Ntfs3G`]: While executing an add command with
	///   [`AddFlags::NTFS`] specified, an error occurred while reading data
	///   from the NTFS volume using libntfs-3g
	/// - [`Error::Open`]: Failed to open a file to be captured while executing
	///   an add command
	/// - [`Error::Opendir`]: Failed to open a directory to be captured while
	///   executing an add command
	/// - [`Error::PathDoesNotExist`]: A delete command without
	///   [`DeleteFlags::FORCE`] specified was for a WIM path that did not
	///   exist; or a rename command attempted to rename a file that does not
	///   exist
	/// - [`Error::Read`]: While executing an add command, failed to read data
	///   from a file or directory to be captured
	/// - [`Error::Readlink`]: While executing an add command, failed to read
	///   the target of a symbolic link, junction, or other reparse point
	/// - [`Error::Stat`]: While executing an add command, failed to read
	///   metadata for a file or directory
	/// - [`Error::UnableToReadCaptureConfig`]: A capture configuration file
	///   could not be read
	/// - [`Error::Unsupported`]: A command had flags provided that are not
	///   supported on this platform or in this build of the library
	/// - [`Error::UnsupportedFile`]: An add command with
	///   [`AddFlags::NO_UNSUPPORTED_EXCLUDE`] specified discovered a file that
	///   was not of a supported type
	///
	/// ## Metadata read-related
	/// - [`Error::Decompression`]
	/// - [`Error::InvalidMetadataResource`]
	/// - [`Error::MetadataNotFound`]
	/// - [`Error::Read`]
	/// - [`Error::UnexpectedEndOfFile`]
	pub fn update(
		&self,
		commands: &[UpdateCommand],
		update_flags: UpdateFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_update_image(
				self.wimstruct,
				self.ffi_index(),
				commands.as_ptr().cast(),
				commands.len(),
				update_flags.bits(),
			)
		})
	}
}

/// Specification of an update to perform on a WIM image
#[derive(Clone, Copy)]
#[repr(transparent)]
pub struct UpdateCommand<'a> {
	inner: sys::wimlib_update_command,
	_borrows: PhantomData<&'a TStr>,
}

impl Debug for UpdateCommand<'_> {
	fn fmt(&self, _f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		// TODO
		Ok(())
	}
}

impl PartialEq for UpdateCommand<'_> {
	fn eq(&self, _other: &Self) -> bool {
		false // TODO
	}
}

impl Eq for UpdateCommand<'_> {}

impl<'a> UpdateCommand<'a> {
	/// Add a new file or directory tree to the image
	///
	/// # Parameters
	/// - `fs_source_path`: Filesystem path to the file or directory tree to add
	/// - `wim_target_path`: Destination path in the image
	/// - `config_file`: Path to capture configuration file to use
	/// - `add_flags`: Extra flags
	pub fn add(
		fs_source_path: &'a TStr,
		wim_target_path: &'a TStr,
		config_file: Option<&'a TStr>,
		add_flags: AddFlags,
	) -> Self {
		let command = sys::wimlib_add_command {
			fs_source_path: fs_source_path.as_ptr().cast_mut(),
			wim_target_path: wim_target_path.as_ptr().cast_mut(),
			config_file: config_file.as_nullable_ptr().cast_mut(),
			add_flags: add_flags.bits(),
		};

		let inner = sys::wimlib_update_command {
			op: sys::wimlib_update_op_WIMLIB_UPDATE_OP_ADD,
			__bindgen_anon_1: sys::wimlib_update_command__bindgen_ty_1 { add: command },
		};

		Self {
			inner,
			_borrows: PhantomData,
		}
	}

	/// Delete a file or directory tree from the image
	///
	/// # Parameters
	/// - `path`: The path to the file or directory within the image to delete
	/// - `delete_flags`: Extra flags
	pub fn delete(path: &'a TStr, delete_flags: DeleteFlags) -> Self {
		let command = sys::wimlib_delete_command {
			wim_path: path.as_ptr().cast_mut(),
			delete_flags: delete_flags.bits(),
		};

		let inner = sys::wimlib_update_command {
			op: sys::wimlib_update_op_WIMLIB_UPDATE_OP_DELETE,
			__bindgen_anon_1: sys::wimlib_update_command__bindgen_ty_1 { delete_: command },
		};

		Self {
			inner,
			_borrows: PhantomData,
		}
	}

	/// Rename a file or directory tree in the image
	/// - `from`: The path to the source file or directory within the image
	/// - `to`: The path to the destination file or directory within the image
	/// - `rename_flags`: Extra flags
	pub fn rename(from: &'a TStr, to: &'a TStr, rename_flags: RenameFlags) -> Self {
		let command = sys::wimlib_rename_command {
			wim_source_path: from.as_ptr().cast_mut(),
			wim_target_path: to.as_ptr().cast_mut(),
			rename_flags: rename_flags.bits(),
		};

		let inner = sys::wimlib_update_command {
			op: sys::wimlib_update_op_WIMLIB_UPDATE_OP_RENAME,
			__bindgen_anon_1: sys::wimlib_update_command__bindgen_ty_1 { rename: command },
		};

		Self {
			inner,
			_borrows: PhantomData,
		}
	}

	/// Convert to raw command
	pub fn into_raw(self) -> sys::wimlib_update_command {
		self.inner
	}

	/// Create from raw command
	///
	/// # Safety
	/// - The structue has to be valid
	pub unsafe fn from_raw(ffi: sys::wimlib_update_command) -> Self {
		Self {
			inner: ffi,
			_borrows: PhantomData,
		}
	}
}

/// Builder for [`Wim::set_info`]
#[derive(Default)]
pub struct SetInfo {
	readonly_flag: Option<bool>,
	guid: Option<Uuid>,
	boot_index: Option<Option<ImageIndex>>,
	rpfix_flag: Option<bool>,
}

impl SetInfo {
	/// Set or unset the »readonly« WIM header flag (`WIM_HDR_FLAG_READONLY` in
	/// Microsoft's documentation)
	///
	/// This is distinct from basic file permissions; this flag can be set on a
	/// WIM file that is physically writable.
	///
	/// wimlib disallows modifying on-disk WIM files with the readonly flag set.
	/// However, [`Wim::overwrite`] with
	/// [`crate::WriteFlags::IGNORE_READONLY_FLAG`] will override this — and in
	/// fact, this is necessary to set the readonly flag persistently on an
	/// existing WIM file.
	pub fn readonly_flag(mut self, flag: bool) -> Self {
		self.readonly_flag = Some(flag);
		self
	}

	/// Set the GUID (globally unique identifier) of the WIM file
	pub fn guid(mut self, guid: Uuid) -> Self {
		self.guid = Some(guid);
		self
	}

	/// Change the bootable image of the WIM
	pub fn boot_index(mut self, boot_index: Option<ImageIndex>) -> Self {
		self.boot_index = Some(boot_index);
		self
	}

	/// Change the `WIM_HDR_FLAG_RP_FIX` flag of the WIM file
	pub fn rpfix_flag(mut self, flag: bool) -> Self {
		self.rpfix_flag = Some(flag);
		self
	}
}

bitflags::bitflags! {
	/// Flags related to operations of adding items to the WIM
	pub struct AddFlags: std::ffi::c_int {
		#[cfg(any(unix, doc))]
		/// Directly capture an NTFS volume rather than a generic directory
		const NTFS                   = sys::WIMLIB_ADD_FLAG_NTFS                   as _;
		/// Follow symbolic links when scanning the directory tree
		const DEREFERENCE            = sys::WIMLIB_ADD_FLAG_DEREFERENCE            as _;
		/// Call the progress function with the message [`ProgressMsg::ScanDentry`]
		/// when each directory or file has been scanned
		const VERBOSE                = sys::WIMLIB_ADD_FLAG_VERBOSE                as _;
		/// Mark the image being added as the bootable image of the WIM
		const BOOT                   = sys::WIMLIB_ADD_FLAG_BOOT                   as _;
		#[cfg(any(unix, doc))]
		/// Store the UNIX owner, group, mode, and device ID (major and minor number)
		/// of each file
		const UNIX_DATA              = sys::WIMLIB_ADD_FLAG_UNIX_DATA              as _;
		/// Do not capture security descriptors
		const NO_ACLS                = sys::WIMLIB_ADD_FLAG_NO_ACLS                as _;
		/// Fail immediately if the full security descriptor of any file or
		/// directory cannot be accessed
		const STRICT_ACLS            = sys::WIMLIB_ADD_FLAG_STRICT_ACLS            as _;
		/// Call the progress function with the message [`ProgressMsg::ScanDentry`]
		/// when a directory or file is excluded from capture
		const EXCLUDE_VERBOSE        = sys::WIMLIB_ADD_FLAG_EXCLUDE_VERBOSE        as _;
		/// Reparse-point fixups: Modify absolute symbolic links (and junctions,
		/// in the case of Windows) that point inside the directory being captured
		/// to instead be absolute relative to the directory being captured
		const RPFIX                  = sys::WIMLIB_ADD_FLAG_RPFIX                  as _;
		/// Don't do reparse point fixups
		const NORPFIX                = sys::WIMLIB_ADD_FLAG_NORPFIX                as _;
		/// Do not automatically exclude unsupported files or directories from
		/// capture, such as encrypted files in NTFS-3G capture mode, or device
		/// files and FIFOs on UNIX-like systems when not also using [`Self::UNIX_DATA`]
		const NO_UNSUPPORTED_EXCLUDE = sys::WIMLIB_ADD_FLAG_NO_UNSUPPORTED_EXCLUDE as _;
		/// Automatically select a capture configuration appropriate for capturing
		/// filesystems containing Windows operating systems
		const WINCONFIG              = sys::WIMLIB_ADD_FLAG_WINCONFIG              as _;
		/// Capture image as »WIMBoot compatible«
		const WIMBOOT                = sys::WIMLIB_ADD_FLAG_WIMBOOT                as _;
		/// If the add command involves adding a non-directory file to a location
		/// at which there already exists a nondirectory file in the image, issue
		/// [`Error::InvalidOverlay`] instead of replacing the file
		const NO_REPLACE             = sys::WIMLIB_ADD_FLAG_NO_REPLACE             as _;
		/// Send [`ProgressMsg::TestFileExclusion`] messages to the progress function
		const TEST_FILE_EXCLUSION    = sys::WIMLIB_ADD_FLAG_TEST_FILE_EXCLUSION    as _;
		/// create a temporary filesystem snapshot of the source directory and
		/// add the files from it
		const SNAPSHOT               = sys::WIMLIB_ADD_FLAG_SNAPSHOT               as _;
		/// Permit the library to discard file paths after the initial scan
		const FILE_PATHS_UNNEEDED    = sys::WIMLIB_ADD_FLAG_FILE_PATHS_UNNEEDED    as _;
	}

	/// Flags related to operations of deleting items from the WIM
	pub struct DeleteFlags: std::ffi::c_int {
		/// Do not issue an error if the path to delete does not exist
		const FORCE     = sys::WIMLIB_DELETE_FLAG_FORCE     as _;
		/// Delete the file or directory tree recursively; if not specified,
		/// an error is issued if the path to delete is a directory
		const RECURSIVE = sys::WIMLIB_DELETE_FLAG_RECURSIVE as _;
	}

	/// Flags related to exporting image to another WIM
	pub struct ExportFlags: std::ffi::c_int {
		/// If a single image is being exported, mark it bootable in the
		/// destination WIM
		///
		/// Alternatively, if [`super::ALL_IMAGES`]` is specified as the
		/// image to export, the image in the source WIM (if any) that
		/// is marked as bootable is also marked as bootable in the
		/// destination WIM.
		const BOOT            = sys::WIMLIB_EXPORT_FLAG_BOOT            as _;

		/// Give the exported image(s) no names
		///
		/// Avoids problems with image name collisions.
		const NO_NAMES        = sys::WIMLIB_EXPORT_FLAG_NO_NAMES        as _;

		/// Give the exported image(s) no descriptions
		const NO_DESCRIPTIONS = sys::WIMLIB_EXPORT_FLAG_NO_DESCRIPTIONS as _;

		/// This advises the library that the program is finished with
		/// the source [`Wim`] and will not attempt to access it after
		/// export, with the exception of dropping the [`Wim`]
		const GIFT            = sys::WIMLIB_EXPORT_FLAG_GIFT            as _;

		/// Mark each exported image as WIMBoot-compatible
		///
		/// # Note
		/// By itself, this does change the destination WIM's compression type,
		/// nor does it add the file `\Windows\System32\WimBootCompress.ini` in
		/// the WIM file. Before writing the destination WIM, it's recommended
		/// to do something like this:
		///
		/// ```ignore
		/// wim.set_output_compression_type(CompressionType::Xpress);
		/// wim.set_output_chunk_size(4096);
		/// image.add_tree(
		/// 	tstr!("myconfig.ini"),
		/// 	tstr!(r"\Windows\System32\WimBootCompres.ini"),
		/// 	AddFlags::empty(),
		/// );
		/// ```
		const WIMBOOT         = sys::WIMLIB_EXPORT_FLAG_WIMBOOT         as _;
	}

	/// Flags related to updating items in file
	pub struct UpdateFlags: std::ffi::c_int {
		/// Send [`ProgressMsg::UpdateBeginCommand`] and
		/// [`ProgressMsg::UpdateEndCommand`]
		const SEND_PROGRESS = sys::WIMLIB_UPDATE_FLAG_SEND_PROGRESS as _;
	}

	/// Item rename flags – currently unused
	pub struct RenameFlags: std::ffi::c_int {}

	/// Reference template file flags – currently unused
	pub struct ReferenceTemplateImageFlags: std::ffi::c_int {}
}
