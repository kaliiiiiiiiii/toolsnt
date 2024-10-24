use {crate::sys, std::fmt::Display};

/// Create a result from C return code
pub const fn result_from_raw(code: i32) -> Result<(), Error> {
	let code = code as u32;

	if code == sys::wimlib_error_code_WIMLIB_ERR_SUCCESS {
		Ok(())
	} else {
		Err(Error::from_raw(code))
	}
}

macro_rules! define_error_enum {
	{ $($variant:ident),* $(,)? } => {
		paste::paste! {
			/// Possible values of the error code returned by many functions in wimlib
			#[allow(missing_docs)]
			#[derive(Clone, Copy, Debug, PartialEq, Eq)]
			#[repr(u32)]
			#[non_exhaustive]
			pub enum Error {
				$(
					$variant =
						sys::[<wimlib_error_code_WIMLIB_ERR_ $variant:snake:upper>],
				)*
				Ntfs3G = sys::wimlib_error_code_WIMLIB_ERR_NTFS_3G,

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
	AlreadyLocked,
	Decompression,
	Fuse,
	GlobHadNoMatches,
	ImageCount,
	ImageNameCollision,
	InsufficientPrivileges,
	Integrity,
	InvalidCaptureConfig,
	InvalidChunkSize,
	InvalidCompressionType,
	InvalidHeader,
	InvalidImage,
	InvalidIntegrityTable,
	InvalidLookupTableEntry,
	InvalidMetadataResource,
	InvalidOverlay,
	InvalidParam,
	InvalidPartNumber,
	InvalidPipableWim,
	InvalidReparseData,
	InvalidResourceHash,
	InvalidUtf16String,
	InvalidUtf8String,
	IsDirectory,
	IsSplitWim,
	Link,
	MetadataNotFound,
	Mkdir,
	Mqueue,
	Nomem,
	Notdir,
	Notempty,
	NotARegularFile,
	NotAWimFile,
	NotPipable,
	NoFilename,
	Open,
	Opendir,
	PathDoesNotExist,
	Read,
	Readlink,
	Rename,
	ReparsePointFixupFailed,
	ResourceNotFound,
	ResourceOrder,
	SetAttributes,
	SetReparseData,
	SetSecurity,
	SetShortName,
	SetTimestamps,
	SplitInvalid,
	Stat,
	UnexpectedEndOfFile,
	UnicodeStringNotRepresentable,
	UnknownVersion,
	Unsupported,
	UnsupportedFile,
	WimIsReadonly,
	Write,
	Xml,
	WimIsEncrypted,
	Wimboot,
	AbortedByProgress,
	UnknownProgressStatus,
	Mknod,
	MountedImageIsBusy,
	NotAMountpoint,
	NotPermittedToUnmount,
	FveLockedVolume,
	UnableToReadCaptureConfig,
	WimIsIncomplete,
	CompactionNotPossible,
	ImageHasMultipleReferences,
	DuplicateExportedImage,
	ConcurrentModificationDetected,
	SnapshotFailure,
	InvalidXattr,
	SetXattr,
}
