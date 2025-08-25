use {crate::sys, std::fmt::Display};

/// Create a result from C return code
pub const fn result_from_raw(code: i32) -> Result<(), Error> {
	if code == sys::wimlib_error_code_WIMLIB_ERR_SUCCESS as i32 {
		Ok(())
	} else {
		Err(Error::from_raw(code as u32))
	}
}

macro_rules! define_error_enum {
	{
		$(
			$(#[$attr:meta])*
			$variant:ident
		),*
		$(,)?
	} => {
		paste::paste! {
			/// Possible values of the error code returned by many functions in wimlib
			#[allow(missing_docs)]
			#[derive(Clone, Copy, Debug, PartialEq, Eq)]
			#[repr(u32)]
			#[non_exhaustive]
			pub enum Error {
				$(
					$(#[$attr])*
					$variant = sys::[<wimlib_error_code_WIMLIB_ERR_ $variant:snake:upper>] as u32,
				)*

				/// NTFS-3G encountered an error (check errno)
				Ntfs3G = sys::wimlib_error_code_WIMLIB_ERR_NTFS_3G as u32,

				#[doc(hidden)]
				__Unknown = u32::MAX,
			}

			impl Error {
				/// Create error from C library's status code
				pub const fn from_raw(code: u32) -> Self {
					match code {
						$(
							sys::[<wimlib_error_code_WIMLIB_ERR_ $variant:snake:upper>]
								=> Self::$variant,
						)*
						sys::wimlib_error_code_WIMLIB_ERR_NTFS_3G => Self::Ntfs3G,
						_ => Self::__Unknown,
					}
				}
			}
		}
	};
}

impl std::error::Error for Error {}
impl Display for Error {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		let error_str = unsafe {
			let str_ptr = sys::wimlib_get_error_string(*self as u32);
			crate::string::TStr::from_ptr(str_ptr)
		};

		write!(f, "{error_str}")
	}
}

define_error_enum! {
	/// The WIM is already locked for writing
	AlreadyLocked,
	/// The WIM contains invalid compressed data
	Decompression,
	/// An error was returned by `fuse_main`
	Fuse,
	/// The provided file glob did not match any files
	GlobHadNoMatches,
	/// Inconsistent image count among the metadata
	/// resources, the WIM header, and/or the XML data
	ImageCount,
	/// Tried to add an image with a name that is already in use
	ImageNameCollision,
	/// The user does not have sufficient privileges
	InsufficientPrivileges,
	/// The WIM file is corrupted (failed integrity check)
	Integrity,
	/// The contents of the capture configuration file were invalid
	InvalidCaptureConfig,
	/// The compression chunk size was unrecognized
	InvalidChunkSize,
	/// The compression type was unrecognized
	InvalidCompressionType,
	/// The WIM header was invalid
	InvalidHeader,
	/// Tried to select an image that does not exist in the WIM
	InvalidImage,
	/// The WIM's integrity table is invalid
	InvalidIntegrityTable,
	/// An entry in the WIM's lookup table is invalid
	InvalidLookupTableEntry,
	/// The metadata resource is invalid
	InvalidMetadataResource,
	/// Conflicting files in overlay when creating a WIM image
	InvalidOverlay,
	/// An invalid parameter was given
	InvalidParam,
	/// The part number or total parts of the WIM is invalid
	InvalidPartNumber,
	/// The pipable WIM is invalid
	InvalidPipableWim,
	/// The reparse data of a reparse point was invalid
	InvalidReparseData,
	/// The SHA-1 message digest of a WIM resource did not match the expected value
	InvalidResourceHash,
	/// A string was not a valid UTF-8 string
	InvalidUtf8String,
	/// A string was not a valid UTF-16 string
	InvalidUtf16String,
	/// One of the specified paths to delete was a directory
	IsDirectory,
	/// The WIM is part of a split WIM, which is not supported for this operation
	IsSplitWim,
	/// Failed to create a hard or symbolic link when extracting a file from the WIM
	Link,
	/// The WIM does not contain image metadata; it only contains file data
	MetadataNotFound,
	/// Failed to create a directory
	Mkdir,
	/// Failed to create or use a POSIX message queue
	Mqueue,
	/// Ran out of memory
	Nomem,
	/// Expected a directory
	Notdir,
	/// Directory was not empty
	Notempty,
	/// One of the specified paths to extract did not correspond to a regular file
	NotARegularFile,
	/// The file did not begin with the magic characters that identify a WIM file
	NotAWimFile,
	/// The WIM is not identified with a filename
	NoFilename,
	/// The WIM was not captured such that it can be applied from a pipe
	NotPipable,
	/// Failed to open a file
	Open,
	/// Failed to open a directory
	Opendir,
	/// The path does not exist in the WIM image
	PathDoesNotExist,
	/// Could not read data from a file
	Read,
	/// Could not read the target of a symbolic link
	Readlink,
	/// Could not rename a file
	Rename,
	/// Unable to complete reparse point fixup
	ReparsePointFixupFailed,
	/// A file resource needed to complete the operation was missing from the WIM
	ResourceNotFound,
	/// The components of the WIM were arranged in an unexpected order
	ResourceOrder,
	/// Failed to set attributes on extracted file
	SetAttributes,
	/// Failed to set reparse data on extracted file
	SetReparseData,
	/// Failed to set file owner, group, or other permissions on extracted file
	SetSecurity,
	/// Failed to set short name on extracted file
	SetShortName,
	/// Failed to set timestamps on extracted file
	SetTimestamps,
	/// The WIM is part of an invalid split WIM
	SplitInvalid,
	/// Could not read the metadata for a file or directory
	Stat,
	/// Unexpectedly reached the end of the file
	UnexpectedEndOfFile,
	/// A Unicode string could not be represented in the current locale's encoding
	UnicodeStringNotRepresentable,
	/// The WIM file is marked with an unknown version number
	UnknownVersion,
	/// The requested operation is unsupported
	Unsupported,
	/// A file in the directory tree to archive was not of a supported type
	UnsupportedFile,
	/// The WIM is read-only (file permissions, header flag, or split WIM)
	WimIsReadonly,
	/// Failed to write data to a file
	Write,
	/// The XML data of the WIM is invalid
	Xml,
	/// The WIM file (or parts of it) is encrypted
	WimIsEncrypted,
	/// Failed to set WIMBoot pointer data
	Wimboot,
	/// The operation was aborted by the library user
	AbortedByProgress,
	/// The user-provided progress function returned an unrecognized value
	UnknownProgressStatus,
	/// Unable to create a special file (e.g. device node or socket)
	Mknod,
	/// There are still files open on the mounted WIM image
	MountedImageIsBusy,
	/// There is not a WIM image mounted on the directory
	NotAMountpoint,
	/// The current user does not have permission to unmount the WIM image
	NotPermittedToUnmount,
	/// The volume must be unlocked before it can be used
	FveLockedVolume,
	/// The capture configuration file could not be read
	UnableToReadCaptureConfig,
	/// The WIM file is incomplete
	WimIsIncomplete,
	/// The WIM file cannot be compacted because of its format,
	/// its layout, or the write parameters specified by the user
	CompactionNotPossible,
	/// The WIM image cannot be modified because it is currently
	/// referenced from multiple places
	ImageHasMultipleReferences,
	/// The destination WIM already contains one of the source images
	DuplicateExportedImage,
	/// A file being added to a WIM image was concurrently modified
	ConcurrentModificationDetected,
	/// Unable to create a filesystem snapshot
	SnapshotFailure,
	/// An extended attribute entry in the WIM image is invalid
	InvalidXattr,
	/// Failed to set an extended attribute on an extracted file
	SetXattr,
}
