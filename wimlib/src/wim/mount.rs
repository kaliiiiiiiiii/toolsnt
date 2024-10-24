//! Mount and unmount WIM images
//!
//! On Linux, wimlib supports mounting images from WIM files either read-only or
//! read-write. To mount an image, call [`Image::mount`]. To unmount an
//! image, call [`WimLib::unmount_image`]. Mounting can be done without root
//! privileges because it is implemented using FUSE (Filesystem in Userspace).
//!
//! If wimlib is compiled using the `–without-fuse` flag, these functions will
//! be available but will fail with [`Error::Unsupported`].
//!
//! # Note
//! if mounting is unsupported, wimlib still provides another way to modify a
//! WIM image ([`Image::update`]).

use crate::{
	error::result_from_raw,
	progress::{trampoline, ProgressCallback},
	string::{OptionalAsFfiExt, TStr},
	sys, Error, Image, WimLib,
};

impl<'a> Image<'a> {
	/// Mount an image from a WIM file on a directory read-only or read-write
	///
	/// # Parameters
	/// - `dir`: The path to an existing empty directory on which to mount the
	///   image `mount_flags`: Extra mount flags. Use [`MountFlags::READWRITE`]
	///   to request a read-write mount isntead of a read-only mount.
	/// - `staging_dir`: If [`Some`], the name of a directory in which a
	///   temporary directory for string modified or added files will be
	///   created. Ignored if [`MountFlags::READWRITE`] is not specified in
	///   `mount_flags`. If [`None`], the staging directory is created in the
	///   same directory as the backing WIM file. The staging directory is
	///   automatically deleted when the image is unmounted.
	///
	/// # Error values
	/// - [`Error::AlreadyLocked`]: Another process is currently modifying the
	///   WIM file
	/// - [`Error::Fuse`]: A non-zero status code was returned by `fuse_main`
	/// - [`Error::ImageHasMultipleReferences`]: There are currently multiple
	///   references to the image as a result of a call to [`Self::export`].
	///   Drop one before attempting the read-write mount.
	/// - [`Error::InvalidImage`]: This image doesn't exist in the backing
	///   [`crate::Wim`]
	/// - [`Error::InvalidParam`]: `dir` was an empty string or the image has
	///   already been modified in memory
	/// - [`Error::Mkdir`]: [`MountFlags::READWRITE`] was specified in
	///   `mount_flags`, but the staging directory could not be created
	/// - [`Error::WimIsReadonly`]: [`MountFlags::READWRITE`] was specified in
	///   `mount_flags`, but the WIM file is considered read-only because of any
	///   of the reasons mentioned in the documentation for the
	///   [`crate::OpenFlags::WRITE_ACCESS`] flag
	/// - [`Error::Unsupported`]: Mounting is not supported in this build of the
	///   library
	/// - Additionally [`Error::Decompression`],
	///   [`Error::InvalidMetadataResource`], [`Error::MetadataNotFound`],
	///   [`Error::Read`] or [`Error::UnexpectedEndOfFile`], all of which
	///   indicate failure (for different reasons) to read the metadata resource
	///   for an image to mount
	///
	/// The ability to mount WIM images is implemented using FUSE (Filesystem in
	/// UserSpacE). Depending on how FUSE is set up on your system, this
	/// function may work as normal users in addition to the root user.
	///
	/// Mounting WIM images is not supported if wimlib was configured
	/// `–without-fuse`. This includes Windows builds of wimlib;
	/// [`Error::Unsupported`] will be returned in such cases.
	///
	/// Calling this function daemonizes the process, unless
	/// [`MountFlags::DEBUG`] was specified or an early error occurs.
	///
	/// It is safe to mount multiple images from the same WIM file read-only at
	/// the same time, but only if different [`crate::Wim`]'s are used. It is
	/// not safe to mount multiple images from the same WIM file read-write at
	/// the same time.
	///
	/// To unmount the image, call [`WimLib::unmount_image`]. This may be done
	/// in a different process.
	pub fn mount(
		&self,
		dir: &TStr,
		mount_flags: MountFlags,
		staging_dir: Option<&TStr>,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_mount_image(
				self.wimstruct,
				self.ffi_index(),
				dir.as_ptr(),
				mount_flags.bits(),
				staging_dir.as_nullable_ptr(),
			)
		})
	}
}

impl WimLib {
	/// Unmount a WIM image that was mounted using [`Image::mount`]
	///
	/// When unmounting a read-write mounted image, the default behavior is to
	/// discard changes to the image. Use [`UnmountFlags::COMMIT`] to cause the
	/// image to be committed.
	///
	/// # Error values
	/// - [`Error::NotAMountpoint`]: There is no WIM image mounted on the
	///   specified directory
	/// - [`Error::MountedImageIsBusy`]: The read-write mounted image cannot be
	///   committed because there are file descriptors open to it, and
	///   [`UnmountFlags::FORCE`] was not specified
	/// - [`Error::Mqueue`]: Could not create a POSIX message queue
	/// - [`Error::NotPermittedToUnmount`]: The image was mounted by a different
	///   user
	/// - [`Error::Unsupported`]: Mounting is not supported in this build of the
	///   library
	///
	/// # Note
	/// You can also unmount the image by using the `umount` system call, or by
	/// using the `umount` or `fusermount` programs. However, you need to call
	/// this function if you want changes to be committed.
	pub fn unmount_image(&self, dir: &TStr, unmount_flags: UnmountFlags) -> Result<(), Error> {
		result_from_raw(unsafe { sys::wimlib_unmount_image(dir.as_ptr(), unmount_flags.bits()) })
	}

	/// Same as [`Self::unmount_image`], but allows specifying a progress
	/// function.
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::UnmountBegin`]
	/// - [`crate::progress::ProgressMsg::WriteStreams`] if changes are commited
	///   from a read-write mount
	pub fn unmount_image_with_progress_callback(
		&self,
		dir: &TStr,
		unmount_flags: UnmountFlags,
		progress_callback: &mut ProgressCallback,
	) -> Result<(), Error> {
		let mut callback_ptr = progress_callback as *mut _;
		let callback_thin: *mut *mut ProgressCallback = &mut callback_ptr;

		result_from_raw(unsafe {
			sys::wimlib_unmount_image_with_progress(
				dir.as_ptr(),
				unmount_flags.bits(),
				Some(trampoline),
				callback_thin.cast(),
			)
		})
	}
}

bitflags::bitflags! {
	/// Flags for [`Image::mount`]
	pub struct MountFlags: std::ffi::c_int {
		/// Mount the WIM image read-write rather than the default of read-only
		const READWRITE                = sys::WIMLIB_MOUNT_FLAG_READWRITE                as _;
		/// Enable FUSE debugging by passing the -d option to `fuse_main`
		const DEBUG                    = sys::WIMLIB_MOUNT_FLAG_DEBUG                    as _;
		/// Do not allow accessing named data streams in the mounted WIM image
		const STREAM_INTERFACE_NONE    = sys::WIMLIB_MOUNT_FLAG_STREAM_INTERFACE_NONE    as _;
		/// Access named data streams in the mounted WIM image through extended
		/// file attributes named "user.X", where X is the name of a data stream.
		///
		/// This is the default mode.
		const STREAM_INTERFACE_XATTR   = sys::WIMLIB_MOUNT_FLAG_STREAM_INTERFACE_XATTR   as _;
		/// Access named data streams in the mounted WIM image by specifying the
		/// file name, a colon, then the name of the data stream.
		const STREAM_INTERFACE_WINDOWS = sys::WIMLIB_MOUNT_FLAG_STREAM_INTERFACE_WINDOWS as _;
		/// Support UNIX owners, groups, modes, and special files
		const UNIX_DATA                = sys::WIMLIB_MOUNT_FLAG_UNIX_DATA                as _;
		/// Allow other users to see the mounted filesystem
		///
		/// This passes the allow_other option to `fuse_main`
		const ALLOW_OTHER              = sys::WIMLIB_MOUNT_FLAG_ALLOW_OTHER              as _;
	}

	/// Flags for [`WimLib::unmount_image`] and [`WimLib::unmount_image_with_progress_callback`]
	pub struct UnmountFlags: std::ffi::c_int {
		/// Provide [`WriteFlags::CHECK_INTEGRITY`] when committing the WIM image
		///
		/// Ignored if [`Self::COMMIT`] not also specified.
		const CHECK_INTEGRITY = sys::WIMLIB_UNMOUNT_FLAG_CHECK_INTEGRITY as _;
		/// Commit changes to the read-write mounted WIM image
		///
		/// If this flag is not specified, changes will be discarded.
		const COMMIT          = sys::WIMLIB_UNMOUNT_FLAG_COMMIT          as _;
		/// Provide [`WriteFlags::REBUILD`] when committing the WIM image
		///
		/// Ignored if [`Self::COMMIT`] not also specified.
		const REBUILD         = sys::WIMLIB_UNMOUNT_FLAG_REBUILD         as _;
		/// Provide [`WriteFlags::RECOMPRESS`] when committing the WIM image
		///
		/// Ignored if [`Self::COMMIT`] not also specified.
		const RECOMPRESSS     = sys::WIMLIB_UNMOUNT_FLAG_RECOMPRESS      as _;
		/// In combination with [`Self::COMMIT`] for a read-write mounted WIM
		/// image, forces all file descriptors to the open WIM image to be
		/// closed before committing it.
		///
		/// Without [`Self::COMMIT`] or with a read-only mounted WIM image,
		/// this flag has no effect.
		const FORCE           = sys::WIMLIB_UNMOUNT_FLAG_FORCE           as _;
		/// In combination with [`Self::COMMIT`] for a read-write mounted WIM
		/// mage, causes the modified image to be committed to the WIM file as
		/// a new, unnamed image appended to the archive.
		///
		/// The original image in the WIM file will be unmodified.
		const NEW_IMAGE       = sys::WIMLIB_UNMOUNT_FLAG_NEW_IMAGE       as _;

	}
}
