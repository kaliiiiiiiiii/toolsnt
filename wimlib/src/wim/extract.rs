//! Extract files, directories, and images from a WIM
//!
//! [`Image::extract`] extracts, or »applies«, an image from a WIM. This
//! normally extracts the image to a directory, but when supported by the build
//! of the library there is also a special NTFS volume extraction mode (entered
//! when [`ExtractFlags::NTFS`] is specified) that allows extracting a WIM
//! image directly to an unmounted NTFS volume. Various other flags allow
//! further customization of image extraction.
//!
//! [`Image::extract_from_paths`] and [`Image::extract_from_pathlist`] allow
//! extracting a list of (possibly wildcard) paths from an WIM image.
//!
//! [`WimLib::extract_image_from_pipe`] extracts an image from a pipable WIM
//! sent over a pipe; see [Pipable WIMs](crate#pipable-wims)
//!
//! Some details of how WIM extraction works are described more fully in the
//! documentation for **wimapply** and **wimextract**.

#[cfg(not(windows))]
use std::os::fd::AsRawFd;
#[cfg(windows)]
use std::os::windows::io::AsRawHandle;

use {
	crate::{
		error::result_from_raw,
		progress::{trampoline, ProgressCallback},
		string::{OptionalAsFfiExt, TStr, ThinTStr},
		sys, Error, Image, WimLib,
	},
	std::fs::File,
};

impl Image<'_> {
	/// Extract an image (or all images) from a WIM
	///
	/// The exact behavior of how wimlib extracts files from a WIM image is
	/// controllable by the `extract_flags` parameter, but there also are
	/// differences depending on the platform (UNIX-like vs Windows). See the
	/// documentation for `wimapply` for more information, including about the
	/// NTFS-3G extraction mode.
	///
	/// # Error values
	/// - [`Error::Decompression`]: The WIM file contains invalid compressed
	///   data
	/// - [`Error::InvalidImage`]: This image doesn't exist in WIM
	/// - [`Error::InvalidMetadataResource`]: The metadata for an image to
	///   extract was invalid
	/// - [`Error::InvalidParam`]: The extraction flags were invalid; more
	///   details may be found in the documentation for the specific extraction
	///   flags that were specified. Or target was an empty string.
	/// - [`Error::InvalidResourceHash`]: The data of a file that needed to be
	///   extracted was corrupt
	/// - [`Error::Link`]: Failed to create a symbolic link or a hard link
	/// - [`Error::MetadataNotFound`]: WIM does not contain image metadata; for
	///   example, it represents a non-first part of a split WIM
	/// - [`Error::Mkdir`]: Failed create a directory
	/// - [`Error::Ntfs3G`]: `libntfs-3g` reported that a problem occurred while
	///   writing to the NTFS volume
	/// - [`Error::Open`]: Could not create a file, or failed to open an
	///   already-extracted file
	/// - [`Error::Read`]: Failed to read data from the WIM.
	/// - [`Error::Readlink`]: Failed to determine the target of a symbolic link
	///   in the WIM
	/// - [`Error::ReparsePointFixupFailed`]: Failed to fix the target of an
	///   absolute symbolic link (e.g. if the target would have exceeded the
	///   maximum allowed length). (Only if reparse data was supported by the
	///   extraction mode and [`ExtractFlags::STRICT_SYMLINKS`] was specified in
	///   extract_flags.)
	/// - [`Error::ResourceNotFound`]: A file data blob that needed to be
	///   extracted could not be found in the blob lookup table of WIM
	/// - [`Error::SetAttributes`]: Failed to set attributes on a file
	/// - [`Error::SetReparseData`]: Failed to set reparse data on a file (only
	///   if reparse data was supported by the extraction mode)
	/// - [`Error::SetSecurity`]: Failed to set security descriptor on a file
	/// - [`Error::SetShortName`]: Failed to set the short name of a file
	/// - [`Error::SetTimestamps`]: Failed to set timestamps on a file
	/// - [`Error::UnexpectedEndOfFile`]: Unexpected end-of-file occurred when
	///   reading data from the WIM
	/// - [`Error::Unsupported`]: A requested extraction flag, or the data or
	///   metadata that must be extracted to support it, is unsupported in the
	///   build and configuration of wimlib, or on the current platform or
	///   extraction mode or target volume
	/// - [`Error::Wimboot`]: [`ExtractFlags::WIMBOOT`] was specified in
	///   extract_flags, but there was a problem creating WIMBoot pointer files
	///   or registering a source WIM file with the Windows Overlay Filesystem
	///   (WOF) driver
	/// - [`Error::Write`]: Failed to write data to a file being extracted
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::ExtractImageBegin`] at the beginning
	/// - [`crate::progress::ProgressMsg::ExtractFileStructure`] (zero or more)
	/// - [`crate::progress::ProgressMsg::ExtractStreams`] (zero or more)
	/// - [`crate::progress::ProgressMsg::ExtractMetadata`] (zero or more)
	/// - [`crate::progress::ProgressMsg::ExtractImageEnd`] at the end
	pub fn extract(&self, target: &TStr, extract_flags: ExtractFlags) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_extract_image(
				self.wimstruct,
				self.ffi_index(),
				target.as_ptr(),
				extract_flags.bits(),
			)
		})
	}

	/// Similar to [`Self::extract_from_paths`] but the paths to extract from
	/// the WIM image are specified in the ASCII, UTF-8, or UTF-16LE text file
	/// named by `path_list_file` which itself contains the list of paths to
	/// use, one per line.
	///
	/// Leading and trailing whitespace is ignored. Empty lines and lines
	/// beginning with the `';'` or `'#'` characters are ignored. No quotes are
	/// needed, as paths are otherwise delimited by the newline character.
	/// However, quotes will be stripped if present.
	///
	/// If `path_list_file` is [`None`], then the pathlist file is read from
	/// standard input.
	///
	/// # Error values
	/// - Refer to [`Self::extract_from_paths`]
	///     - Except appropriate error values returned when path list file can't
	///       be read
	pub fn extract_from_pathlist(
		&self,
		target: &TStr,
		path_list_file: Option<&TStr>,
		extract_flags: ExtractFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_extract_pathlist(
				self.wimstruct,
				self.ffi_index(),
				target.as_ptr(),
				path_list_file.as_nullable_ptr(),
				extract_flags.bits(),
			)
		})
	}

	/// Extract zero or more paths (files or directory trees) from the specified
	/// WIM image
	///
	/// By default, each path will be extracted to a corresponding subdirectory
	/// of the target based on its location in the image. For example, if one of
	/// the paths to extract is `/Windows/explorer.exe` and the target is
	/// `outdir`, the file will be extracted to `outdir/Windows/explorer.exe`.
	/// This behavior can be changed by providing the flag
	/// [`ExtractFlags::NO_PRESERVE_DIR_STRUCTURE`], which will cause each
	/// file or directory tree to be placed directly in the target directory —
	/// so the same example would extract `/Windows/explorer.exe` to
	/// `outdir/explorer.exe`.
	///
	/// With globbing turned off (the default), paths are always checked for
	/// existence strictly; that is, if any path to extract does not exist in
	/// the image, then nothing is extracted and the function fails with
	/// [`Error::PathDoesNotExist`]. But with globbing turned on
	/// ([`ExtractFlags::GLOB_PATHS`] specified), globs are by default
	/// permitted to match no files, and there is a flag
	/// ([`ExtractFlags::STRICT_GLOB`]) to enable the strict behavior if
	/// desired.
	///
	/// Symbolic links are not dereferenced when paths in the image are
	/// interpreted.
	///
	/// # Paths
	/// Slice of paths to extract. Each element must be the absolute path to a
	/// file or directory within the image. Path separators may be either
	/// forwards or backwards slashes, and leading path separators are optional.
	/// The paths will be interpreted either case-sensitively (UNIX default) or
	/// case-insensitively (Windows default); however, the case sensitivity can
	/// be configured explicitly at library initialization time by passing an
	/// appropriate flag to [`WimLib::try_init`].
	/// By default, »globbing« is disabled, so the characters `*` and `?` are
	/// interpreted literally. This can be changed by specifying
	/// [`ExtractFlags::GLOB_PATHS`] flag.
	///
	/// # Error values
	/// - See [`Self::extract`]
	/// - [`Error::NotARegularFile`]: [`ExtractFlags::TO_STDOUT`] was specified
	///   in `extract_flags`, but one of the paths to extract did not name a
	///   regular file.
	/// - [`Error::PathDoesNotExist`]: One of the paths to extract does not
	///   exist in the image; see discussion above about strict vs. non-strict
	///   behavior.
	///
	/// # Progress msessages
	/// - [`crate::progress::ProgressMsg::ExtractStreams`]
	pub fn extract_from_paths(
		&self,
		paths: &[ThinTStr],
		target: &TStr,
		extract_flags: ExtractFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_extract_paths(
				self.wimstruct,
				self.ffi_index(),
				target.as_ptr(),
				paths.as_ptr().cast(),
				paths.len(),
				extract_flags.bits(),
			)
		})
	}
}

impl WimLib {
	/// Extract one image from a pipe on which a pipable WIM is being sent
	///
	/// See the documentation for [`crate::WriteFlags::PIPABLE`] and
	/// [Pipable WIMs](crate#pipable-wims) for more information about pipable
	/// WIMs.
	///
	/// This function operates in a special way to read the WIM fully
	/// sequentially. As a result, there is no [`crate::Wim`] visible to library
	/// users, and you cannot call [`WimLib::open_wim`] on the pipe. (You can,
	/// however, use [`WimLib::open_wim`] to transparently open a pipable WIM if
	/// it's available as a seekable file, not a pipe.)
	///
	/// # Parameters
	/// - `pipe`: File, which may be a pipe, opened for reading and positioned
	///   at the start of the pipable WIM
	/// - `image_num_or_name`: Specified 1-based image or name of the image to
	///   extract. Uses same rules as [`crate::Wim::resolve_image`]. Only a
	///   single image can be specified. ALternatively, specify [`None`] here to
	///   use the first image in the WIM if it contains exactly one image, but
	///   otherwise return [`Error::InvalidImage`]
	/// - `target`: See [`Image::extract`]
	/// - `extract_flags`: See [`Image::extract`]
	///
	/// # Error values
	/// - Inherits from [`Self::open_wim`] and [`Image::extract`]
	/// - [`Error::InvalidPipableWim`]: Data read from the pipable WIM was
	///   invalid
	/// - [`Error::NotPipable`]: The WIM being piped over is a normal WIM, not a
	///   pipable WIM.
	pub fn extract_image_from_pipe(
		&self,
		pipe: &File,
		image_num_or_name: Option<&TStr>,
		target: &TStr,
		extract_flags: ExtractFlags,
	) -> Result<(), Error> {
		#[cfg(not(windows))]
		let raw_handle = file.as_raw_fd() as std::os::raw::c_int;
		#[cfg(windows)]
		let raw_handle = file.as_raw_handle() as std::os::raw::c_int;

		result_from_raw(unsafe {
			sys::wimlib_extract_image_from_pipe(
				raw_handle,
				image_num_or_name.as_nullable_ptr(),
				target.as_ptr(),
				extract_flags.bits(),
			)
		})
	}

	/// Same as [`Self::extract_image_from_pipe`], but allows specifying a
	/// progress function
	///
	/// The progress function will be used while extracting the image and will
	/// receive the normal extraction progress messages, such as
	/// [`crate::progress::ProgressMsg::ExtractStreams`], in addition to
	/// [`crate::progress::ProgressMsg::ExtractSpwmPartBegin`].
	pub fn extract_image_from_pipe_with_progress_callback(
		&self,
		pipe: &File,
		image_num_or_name: Option<&TStr>,
		target: &TStr,
		extract_flags: ExtractFlags,
		progress_callback: &mut ProgressCallback,
	) -> Result<(), Error> {
		#[cfg(not(windows))]
		let raw_handle = file.as_raw_fd() as std::os::raw::c_int;
		#[cfg(windows)]
		let raw_handle = file.as_raw_handle() as std::os::raw::c_int;

		let callback_thin: *mut *mut ProgressCallback = &mut (progress_callback as *mut _);

		result_from_raw(unsafe {
			sys::wimlib_extract_image_from_pipe_with_progress(
				raw_handle,
				image_num_or_name.as_nullable_ptr(),
				target.as_ptr(),
				extract_flags.bits(),
				Some(trampoline),
				callback_thin.cast(),
			)
		})
	}
}

bitflags::bitflags! {
	/// Flags to configure extraction methods for [`Image`]
	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	pub struct ExtractFlags: std::ffi::c_int {
		/// Extract the image directly to an NTFS volume rather than a generic
		/// directory
		///
		/// This mode is only available if wimlib was compiled with `libntfs-3g`
		/// support; if not, [`Error::Unsupported`] will be returned. In this mode,
		/// the extraction target will be interpreted as the path to an NTFS
		/// volume image (as a regular file or block device) rather than a
		/// directory. It will be opened using libntfs-3g, and the image will
		/// be extracted to the NTFS filesystem's root directory. Note: this flag
		/// cannot be used when [`Image::extract_image`] is called with
		/// [`super::ALL_IMAGES`] as the image, nor can it be used with
		/// [`Image::extract_paths`] when passed multiple paths.
		const NTFS                     = sys::WIMLIB_EXTRACT_FLAG_NTFS                       as _;

		/// Don't consider corrupted files to be an error
		const RECOVER_DATA             = sys::WIMLIB_EXTRACT_FLAG_RECOVER_DATA               as _;

		#[cfg(any(unix, doc))]
		/// Extract UNIX-specific metadata captured with
		/// [`super::modify::AddFlags::UNIX_DATA`]
		const UNIX_DATA                = sys::WIMLIB_EXTRACT_FLAG_UNIX_DATA                  as _;

		/// Do not extract security descriptors
		///
		/// This flag cannot be combined with [`Self::STRICT_ACLS`].
		const NO_ACLS                  = sys::WIMLIB_EXTRACT_FLAG_NO_ACLS                    as _;

		/// Fail immediately if the full security descriptor of any file or
		/// directory cannot be set exactly as specified in the WIM image
		///
		/// On Windows, the default behavior without this flag when wimlib does
		/// not have permission to set the correct security descriptor is to
		/// fall back to setting the security descriptor with the SACL omitted,
		/// then with the DACL omitted, then with the owner omitted, then not at
		/// all. This flag cannot be combined with [`Self::NO_ACLS`].
		const STRICT_ACLS              = sys::WIMLIB_EXTRACT_FLAG_STRICT_ACLS                as _;

		/// This is the extraction equivalent to [`super::modify::AddFlags::RPFIX`]
		///
		/// This forces reparse-point fixups on, so absolute symbolic links or
		/// junction points will be fixed to be absolute relative to the actual
		/// extraction root. Reparse-point fixups are done by default for
		/// [`Image::extract`] and [`Image::extract_from"pipe`] if
		/// `WIM_HDR_FLAG_RP_FIX` is set in the WIM header. This flag cannot be
		/// combined with [`Self::NORPFIX`].
		const RPFIX                    = sys::WIMLIB_EXTRACT_FLAG_RPFIX                      as _;

		/// Force reparse-point fixups on extraction off, regardless of the state
		/// of the `WIM_HDR_FLAG_RP_FIX` flag in the WIM header
		const NORPFIX                  = sys::WIMLIB_EXTRACT_FLAG_NORPFIX                    as _;

		/// For [`Image::extract_paths`] and [`Image::extract_pathlist`]
		/// only. Extract the paths, each of which must name a regualr file, to standard
		/// output.
		const TO_STDOUT                = sys::WIMLIB_EXTRACT_FLAG_TO_STDOUT                  as _;

		/// Instead of ignoring files and directories with names that cannot be
		/// represented on the current platform (note: Windows has more
		/// restrictions on filenames than POSIX-compliant systems), try to
		/// replace characters or append junk to the names so that they can be
		/// extracted in some form
		///
		/// # Note
		/// This flag is unlikely to have any effect when extracting a WIM image
		/// that was captured on Windows.
		const REPLACE_INVALID_FILENAMES = sys::WIMLIB_EXTRACT_FLAG_REPLACE_INVALID_FILENAMES  as _;

		/// On Windows, when there exist two or more files with the same case
		/// insensitive name but different case sensitive names, try to extract
		/// them all by appending junk to the end of them, rather than
		/// arbitrarily extracting only one.
		const ALL_CASE_CONFLICTS        = sys::WIMLIB_EXTRACT_FLAG_ALL_CASE_CONFLICTS         as _;

		#[cfg(any(unix, doc))]
		/// Do not ignore failure to set timestamps on extracted files
		///
		/// This flag currently only has an effect when extracting to a directory
		/// on UNIX-like systems
		const STRICT_TIMESTAMPS         = sys::WIMLIB_EXTRACT_FLAG_STRICT_TIMESTAMPS          as _;

		#[cfg(any(windows, doc))]
		/// Do not ignore failure to set short names on extracted files
		///
		/// This flag currently only has an effect on Windows
		const STRICT_SHORT_NAMES        = sys::WIMLIB_EXTRACT_FLAG_STRICT_SHORT_NAMES         as _;

		#[cfg(any(windows, doc))]
		/// Do not ignore failure to extract symbolic links and junctions due
		/// to permissions problems
		///
		/// This flag currently only has an effect on Windows. By default, such
		/// failures are ignored since the default configuration of Windows only
		/// allows the Administrator to create symbolic links.
		const STRICT_SYMLINKS           = sys::WIMLIB_EXTRACT_FLAG_STRICT_SYMLINKS            as _;

		/// For [`Image::extract_paths`] and [`Image::extract_pathlist`]
		/// only: Treat the paths to extract as wildcard patterns (»globs«) which
		/// may contain the wildcard characters `?` and `*`.
		///
		/// The `?` character matches any non-path-separator character, whereas
		/// the `*` character matches zero or more non-path-separator characters.
		/// Consequently, each glob may match zero or more actual paths in the
		/// WIM image.
		///
		/// By default, if a glob does not match any files, a warning but not an
		/// error will be issued. This is the case even if the glob did not
		/// actually contain wildcard characters.
		/// Use [`Self::STRICT_GLOB`] to get an error instead.
		const GLOB_PATHS                = sys::WIMLIB_EXTRACT_FLAG_GLOB_PATHS                as _;

		/// In combination with [`Self::GLOB_PATHS`], causes an error
		/// ([`Error::PathDoesNotExist`]) rather than a warning to be issued when
		/// one of the provided globs did not match a file
		const STRICT_GLOB               = sys::WIMLIB_EXTRACT_FLAG_STRICT_GLOB               as _;

		/// For [`Image::extract_paths`] and
		/// [`Image::extract_pathlist`] only: Do not preserve the directory
		/// structure of the archive when extracting — that is, place each
		/// extracted file or directory tree directly in the target directory
		///
		/// The target directory will still be created if it does not already exist.
		const NO_PRESERVE_DIR_STRUCTURE = sys::WIMLIB_EXTRACT_FLAG_NO_PRESERVE_DIR_STRUCTURE as _;

		#[cfg(any(windows, doc))]
		/// Extract files as »pointers« back to the WIM archive
		///
		/// The effects of this option are failry complex. See the documentation
		/// of the `-wimboot` option of `wimapply` for more information.
		const WIMBOOT                   = sys::WIMLIB_EXTRACT_FLAG_WIMBOOT                   as _;

		/// Compress the extracted files using System Compression, when possible
		///
		/// This only works on either Windows 10 or later, or on an older Windows
		/// to which Microsoft's wofadk.sys driver has been added. Several
		/// different compression formats may be used with System Compression;
		/// this particular flag selects the XPRESS compression format with
		/// 4 KiB chunks.
		const COMPACT_XPRESS4K          = sys::WIMLIB_EXTRACT_FLAG_COMPACT_XPRESS4K          as _;

		/// Like [`Self::COMPACT_XPRESS4K`], but use XPRESS compression with 8 KiB chunks
		const COMPACT_XPRESS8K          = sys::WIMLIB_EXTRACT_FLAG_COMPACT_XPRESS8K          as _;
		/// Like [`Self::COMPACT_XPRESS4K`], but use XPRESS compression with 16 KiB chunks
		const COMPACT_XPRESS16K         = sys::WIMLIB_EXTRACT_FLAG_COMPACT_XPRESS16K         as _;
		/// Like [`Self::COMPACT_XPRESS4K`], but use LZX compression with 32 KiB chunks
		const COMPACT_LZX               = sys::WIMLIB_EXTRACT_FLAG_COMPACT_LZX               as _;
	}
}
