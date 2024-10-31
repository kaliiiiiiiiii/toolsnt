//! Track the progress of long WIM operations
//!
//! Library users can provide a progress function which will be called
//! periodically during operations such as extracting a WIM image or writing a
//! WIM image. A [`crate::Wim`] can have a progress function of type
//! [`ProgressCallback`] associated with it by calling
//! [`crate::Wim::register_progress_callback`] or by opening the [`crate::Wim`]
//! using [`crate::WimLib::open_wim_with_progress_callback`]. Once this is done,
//! the progress function will be called automatically during many operations,
//! such as [`crate::Image::extract`] and [`crate::Image::write`].
//!
//! Some functions that do not operate directly on a user-provided
//! [`crate::Wim`], such as [`crate::WimLib::join`],  also take the progress
//! function directly using an extended version of the function, such as
//! [`crate::WimLib::join_with_progress_callback`].
//!
//! Since wimlib v1.7.0, progress functions are no longer just unidirectional.
//! You can now return [`ProgressStatus::Abort`] to cause the current operationg
//! to be aborted.

use {
	crate::{string::TStr, sys, CompressionType, ExtractFlags, ImageIndex, UpdateCommand},
	derive_more::{Deref, DerefMut},
	std::{ffi, fmt::Debug, num::NonZero},
};

/// Callback for progress
pub type ProgressCallback = dyn FnMut(&mut ProgressMsg) -> ProgressStatus;

pub(crate) unsafe extern "C" fn trampoline(
	tag: sys::wimlib_progress_msg,
	payload: *mut sys::wimlib_progress_info,
	ctx: *mut ffi::c_void,
) -> sys::wimlib_progress_status {
	unsafe {
		let mut msg = ProgressMsg::from_raw(tag, payload).expect("Library returned invalid data");
		let callback_ptr = ctx as *mut *mut ProgressCallback;
		let status = (**callback_ptr)(&mut msg);

		status as _
	}
}

/// Decide whatever should happen after returning from the callback
#[repr(u32)]
pub enum ProgressStatus {
	/// Proceed
	Continue = sys::wimlib_progress_status_WIMLIB_PROGRESS_STATUS_CONTINUE,
	/// Abort the process
	Abort = sys::wimlib_progress_status_WIMLIB_PROGRESS_STATUS_ABORT,
}

/// Progress message
#[derive(Debug)]
pub enum ProgressMsg<'a> {
	/// A WIM image is about to be extracted
	///
	/// This message is received once per image for calls to
	/// [`crate::Image::extract`] and [`crate::WimLib::extract_image_from_pipe`]
	ExtractImageBegin(ExtractMsg<'a>),
	/// One or more file or directory trees within a WIM image is about to be
	/// extracted
	///
	/// This message is received only once per
	/// [`crate::Image::extract_from_paths`] and
	/// [`crate::Image::extract_from_pathlist`], since wimlib combines all paths
	/// into a single execution operation for optimization purposes.
	ExtractTreeBegin(ExtractMsg<'a>),
	/// This message may be sent periodically (not for every file) while files
	/// and directories are being created, prior to file data extraction
	///
	/// `current_file_count` and `end_file_count` may be used to track the
	/// progress of this phase of execution.
	ExtractFileStructure(ExtractMsg<'a, ExtractFsOrMetadataExtras>),
	/// File data is currently being extracted
	///
	/// This is the main message to track the progress of an extraction
	/// operation.
	ExtractStreams(ExtractMsg<'a>),
	/// Starting to read a new part of a split pipable WIM over the pipe
	ExtractSpwmPartBegin(ExtractMsg<'a, ExtractSpwmPartBeginExtras>),
	/// This message may be sent periodically (not necessarily for every file)
	/// while file and directory metadata is being extracted, following file
	/// data extraction
	///
	/// `current_file_count` and `end_file_count` may be used to track the
	/// progress of this phase of execution.
	ExtractMetadata(ExtractMsg<'a, ExtractFsOrMetadataExtras>),
	/// The image has been successfully extracted.
	///
	/// This is paired with [`Self::ExtractImageBegin`]
	ExtractImageEnd(ExtractMsg<'a>),
	/// The files or directory trees have been successfully extracted.
	///
	/// This is paired with [`Self::ExtractTreeBegin`]
	ExtractTreeEnd(ExtractMsg<'a>),
	/// The directory or NTFS volume is about to be scanned for metadata
	///
	/// This message is received once per call to [`crate::Wim::add_image`],
	/// once per capture source passed to [`crate::Wim::add_image_multisource`]
	/// or once per add command passed to [`crate::Image::update`].
	ScanBegin(ScanMsg<'a>),
	/// A directory or file has been scanned
	///
	/// This is paried with a previous [`Self::ScanBegin`].
	///
	/// This message is oly sent if [`crate::AddFlags::VERBOSE`] has been
	/// specified.
	ScanDentry {
		/// Scan message
		scan: ScanMsg<'a>,
		/// Path to the file (or directory) that has been scanned
		cur_path: &'a TStr,
		/// Status
		status: ScanDentryStatus,
	},
	/// The directory or NTFS volume has been successfully scanned
	///
	/// This is paired with a previous [`Self::ScanBegin`] message, possibly
	/// with many intervening [`Self::ScanDentry`] messages.
	ScanEnd(ScanMsg<'a>),
	/// File data is currently being written to the WIM
	///
	/// This message may be received many times while the WIM file is being
	/// written or appended to with [`crate::Image::write`],
	/// [`crate::Wim::overwrite`] or [`crate::Image::write_to_file`]. Since
	/// wimlib v1:13.4 it will also be received when a split WIM part is being
	/// written by [`crate::Wim::split`].
	WriteStreams {
		/// An upper bound on the number of bytes of file data that will be
		/// written
		total_bytes: u64,
		/// An upper bound on the number of distinct file data »blobs« that will
		/// be written
		total_streams: u64,
		/// The number of bytes of file data that have been written so far
		completed_bytes: u64,
		/// The number of distinct file data »blobs« that have been written so
		/// far
		completed_streams: u64,
		/// The number of threads being used for data compression; or, if no
		/// compression is being performed, this will be 1
		num_threads: u32,
		/// The compression type being used
		compression_type: CompressionType,
		/// The number of on-disk WIM files from which file data is being
		/// exported into the output WIM file
		total_parts: u32,
		/// This is currently broken and will always be 0
		completed_parts: u32,
		/// Like `completed_bytes`, but counts the compressed size
		completed_compression_bytes: u64,
	},
	/// Per-image metadata is about to be written to the WIM file
	WriteMetadataBegin,
	/// The per-image metadata has been written to the WIM file
	///
	/// This message is paired with a preceding [`Self::WriteMetadataBegin`]
	WriteMetadataEnd,
	/// [`crate::Wim::overwrite`] has successfully renamed the temporary file to
	/// the original WIM file, thereby committing the changes to the WIM file.
	///
	/// # Note
	/// This message is not recieved if [`crate::Wim::overwrite`] chose to
	/// append to the WIM file in-place.
	Rename {
		/// Name of the temporary file that the WIM was written to
		from: &'a TStr,
		/// Name of the original WIM file to which the temporary file is being
		/// renamed
		to: &'a TStr,
	},
	/// The contents of the WIM file are being checked against the integrity
	/// table
	///
	/// This message is only received (and may be received many times) when
	/// [`crate::WimLib::open_wim_with_progress_callback`] is called with the
	/// [`crate::OpenFlags::CHECK_INTEGRITY`] flag.
	VerifyIntegrity {
		/// Outcome
		calc: IntegrityMsg,
		/// Path to the WIM file being checked
		filename: &'a TStr,
	},
	/// An integrity table is being calculated for the WIM being written
	///
	/// This message is only received (and may be received many times) when a
	/// WIM file is being written with the flag
	/// [`crate::WriteFlags::CHECK_INTEGRITY`].
	CalcIntegrity(IntegrityMsg),
	/// A [`crate::Wim::split`] operation is in progress, and a new split part
	/// is about to be started
	SplitBeginPart(SplitMsg<'a>),
	/// A [`crate::Wim::split`] operation is in progress, and a split part has
	/// been finished.
	SplitEndPart(SplitMsg<'a>),
	/// A WIM update command is about to be executed
	///
	/// This message is received once per update command when
	/// [`crate::Image::update`] is called with the flag
	/// [`crate::UpdateFlags::SEND_PROGRESS`].
	UpdateBeginCommand(UpdateMsg<'a>),
	/// A WIM update command has been executed
	///
	/// This message is received once per update command when
	/// [`crate::Image::update`] is called with the flag
	/// [`crate::UpdateFlags::SEND_PROGRESS`].
	UpdateEndCommand(UpdateMsg<'a>),
	/// A file in the image is being replaced as a result of a
	/// [`crate::UpdateCommand::add`] without [`crate::AddFlags::NO_REPLACE`]
	/// specified.
	///
	/// This is only received when [`crate::AddFlags::VERBOSE`] is also
	/// specified in the add command.
	ReplaceFileInWim {
		/// Path to the file in the image that is being replaced
		path: &'a TStr,
	},
	/// An image is being extracted with [`crate::ExtractFlags::WIMBOOT`], and a
	/// file is being extracted normally (not as a »WIMBoot pointer file«) due
	/// to it matching a pattern in the `[PrepopulateList]` section of the
	/// configuration file /Windows/System32/WimBootCompress.ini in the WIM
	/// image.
	WimbootExclude {
		/// Path to the file in the image
		path_in_wim: &'a TStr,
		/// Path to which the file is being extracted
		extraction_path: &'a TStr,
	},
	#[cfg(feature = "mount")]
	/// Starting to unmount an image
	UnmountBegin {
		/// Path to directory being unmounted
		mountpoint: &'a TStr,
		/// Path to WIM file being unmounted
		mounted_wim: &'a TStr,
		/// 1-based index of image being unmounted
		mounted_image: ImageIndex,
		/// Flags that were passed to [`crate::Image::mount`] when the
		/// mountpoint was set up
		mount_flags: u32,
		/// Flags passed to [`crate::WimLib::unmount_image`]
		unmount_flags: u32,
	},
	/// wimlib has used a file's data for the last time (including all data
	/// streams, if it has multiple)
	///
	/// This message is only received if
	/// [`crate::WriteFlags::SEND_DONE_WITH_FILE_MESSAGES`] was provided.
	DoneWithFile {
		/// Path to the file whose data has been written to the WIM file, or is
		/// currently being asynchronously compressed in memory, and therefore
		/// is no longer needed by wimlib.
		path_to_file: &'a TStr,
	},
	/// [`crate::Wim::verify`] is starting to verify the metadata for an image
	BeginVerifyImage(VerifyImageMsg<'a>),
	/// [`crate::Wim::verify`] has finished verifying the metadata for an
	/// image
	EndVerifyImage(VerifyImageMsg<'a>),
	/// [`crate::Wim::verify`] is verifying file data integrity
	VerifyStreams {
		/// Wim file path
		wimfile: &'a TStr,
		/// Total streams in WIM
		total_streams: u64,
		/// Total bytes
		total_bytes: u64,
		/// Completed streams
		completed_streams: u64,
		/// Completed bytes
		completed_bytes: u64,
	},
	/// The progress function is being asked whether a file should be excluded
	/// from capture or not
	///
	/// Message is bidirectional – setting `will_execute` decides if it should
	/// be excluded.
	///
	/// This message is only received if the flag
	/// [`crate::AddFlags::TEST_FILE_EXCLUSION`] is used. This method for file
	/// exclusions is independent of the »capture configuration file« mechanism
	TestFileExclusion {
		/// Path to file
		path: &'a TStr,
		/// Set this to decide if it should or should not be excluded (defaults
		/// to [`false`])
		will_exclude: &'a mut bool,
	},
	/// An error has occurred and the progress function is being asked whether
	/// to ignore the error or not
	///
	/// This message provides a limited capability for applications to recover
	/// from »unexpected« errors (i.e. those with no in-library handling policy)
	/// arising from the underlying operating system. Normally, any such error
	/// will cause the library to abort the current operation. By implementing a
	/// handler for this message, the application can instead choose to ignore a
	/// given error.
	///
	/// Currently, only the following types of errors will result in this
	/// progress message being sent:
	/// - Directory tree scan errors, e.g. from [`crate::Wim::add_image`]
	/// - Most extraction errors; currently restricted to the Windows build of
	///   the library only
	HandleError {
		/// Path to the file for which the error occurred, or [`None`] if not
		/// relevant
		path: Option<&'a TStr>,
		/// The wimlib error code associated with the error
		error_code: i32,
		/// Indicates whether the error will be ignored or not (defaults to
		/// [`false`])
		will_ignore: &'a mut bool,
	},
}

/// Image extreaction message
///
/// Has extras for different messages
#[derive(Clone, Copy, Debug, Deref, DerefMut, PartialEq, Eq)]
pub struct ExtractMsg<'a, Extras = ()> {
	/// The 1-based index of the image from which files are being extracted
	pub image: ImageIndex,
	/// If the [`crate::Wim`] from which the extraction being performed has a
	/// backing file, then this is an absolute path to that backing file.
	pub wimfile_name: Option<&'a TStr>,
	/// Name of the image from which files are being extracted, or the empty
	/// string if the image is unnamed
	pub image_name: &'a TStr,
	/// Path to the directory or NTFS volume to which the files are being
	/// extracted
	pub target: &'a TStr,
	/// The number of bytes of file data that will be extracted
	pub total_bytes: u64,
	/// The number of bytes of file data that have been extracted so far
	pub completed_bytes: u64,

	/// Specific data
	#[deref]
	#[deref_mut]
	pub extras: Extras,
	extract_flags: ExtractFlags,
}

/// Extras for [`ExtractMsg`] of [`ProgressMsg::ExtractSpwmPartBegin`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractSpwmPartBeginExtras {
	/// Number of the part
	pub part_number: u32,
	/// Total count of parts
	pub total_parts: u32,
}

/// Extras for [`ExtractMsg`] of [`ProgressMsg::ExtractFileStructure`] and
/// [`ProgressMsg::ExtractMetadata`]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExtractFsOrMetadataExtras {
	/// Number of files that have been processed so far
	pub current_file_count: u64,
	/// Total number of files that will be processed
	pub end_file_count: u64,
}

/// Message data for integrity checking-related messages
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct IntegrityMsg {
	/// The number of bytes in the WIM file that are covered by integrity checks
	pub total_bytes: u64,
	/// The number of bytes that have been checksummed so far
	pub completed_bytes: u64,
	/// The number of individually checksummed »chunks« the integrity-checked
	/// region is divided into
	pub total_chunks: u32,
	/// The number of chunks that have been checksummed so far
	pub completed_chunks: u32,
	/// The size of each individually checksummed "chunk" in the
	/// integrity-checked region
	pub chunk_size: u32,
}

/// Image splitting message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SplitMsg<'a> {
	/// Total size of the original WIM's file and metadata resources
	/// (compressed)
	pub total_bytes: u64,
	/// Number of bytes of file and metadata resources that have been copied out
	/// of the original WIM so far
	pub completed_bytes: u64,
	/// Number of the split WIM part that is about to be started
	/// ([`ProgressMsg::SplitBeginPart`]) or has just been finished
	/// ([`ProgressMsg::SplitEndPart`])
	pub cur_part_number: u32,
	/// Total number of split WIM parts that are being written
	pub total_parts: u32,
	/// Name of the split WIM part that is about to be started
	/// ([`ProgressMsg::SplitBeginPart`]) or has just been finished
	/// ([`ProgressMsg::SplitEndPart`])
	pub part_name: &'a TStr,
}

impl SplitMsg<'_> {
	/// Create message from FFI value
	pub fn from_raw(ffi: sys::wimlib_progress_info_wimlib_progress_info_split) -> Self {
		Self {
			total_bytes: ffi.total_bytes,
			completed_bytes: ffi.completed_bytes,
			cur_part_number: ffi.cur_part_number,
			total_parts: ffi.total_parts,
			part_name: unsafe { TStr::from_ptr(ffi.part_name) },
		}
	}
}

/// Scan message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanMsg<'a> {
	/// Top-level directory being scanned; or, when capturing an NTFS volume
	/// with [`crate::AddFlags::NTFS`], this is instead the path to the file or
	/// block device that contains the NTFS volume being scanned.
	pub source: &'a TStr,
	/// Target path in the image or for [`ProgressMsg::ScanDentry`] and a status
	/// of [`ScanDentryStatus::FixedSymlink`],
	/// [`ScanDentryStatus::NotFixedSymlink`], this is the target of the
	/// absolute symbolic link or junction.
	pub target_path: &'a TStr,
	/// The number of directories scanned so far, not counting
	/// excluded/unsupported files
	pub num_dirs_scanned: u64,
	/// The number of non-directories scanned so far, not counting
	/// excluded/unsupported files
	pub num_nondirs_scanned: u64,
	/// The number of bytes of file data detected so far, not counting
	/// excluded/unsupported files
	pub num_bytes_scanned: u64,
}

/// Dentry scan status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScanDentryStatus {
	/// File looks okay and will be captured
	Ok,
	/// File is being excluded from capture due to the capture configuration
	Exluded,
	/// File is being excluded from capture due to being of an unsupported type
	Unsupported,
	/// The file is an absolute symbolic link or junction that points into the
	/// capture directory, and reparse-point fixups are enabled, so its target
	/// is being adjusted.
	///
	/// (Reparse point fixups can be disabled with the flag
	/// [`crate::AddFlags::NORPFIX`])
	FixedSymlink,
	/// Reparse-point fixups are enabled, but the file is an absolute symbolic
	/// link or junction that does **not** point into the capture directory, so
	/// its target is **not** being adjusted.
	NotFixedSymlink,
}

/// Message related to updating image
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UpdateMsg<'a> {
	/// Update command that will be executed or has just been executed
	pub command: &'a UpdateCommand<'a>,
	/// Number of update commands that have been completed so far
	pub completed_commands: usize,
	/// Number of update commands that are being executed as part of this call
	/// to [`crate::Image::update`]
	pub total_commands: usize,
}

impl<'a> UpdateMsg<'a> {
	const fn from_raw(payload: sys::wimlib_progress_info_wimlib_progress_info_update) -> Self {
		Self {
			command: unsafe {
				&*(payload.command.cast::<UpdateMsg>() as *const crate::UpdateCommand<'a>)
			},
			completed_commands: payload.completed_commands,
			total_commands: payload.total_commands,
		}
	}
}

/// Image verification message
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VerifyImageMsg<'a> {
	/// WIM path
	pub wimfile: &'a TStr,
	/// Total images in processing
	pub total_images: u32,
	/// Currently processed iamge
	pub current_image: u32,
}

impl ProgressMsg<'_> {
	unsafe fn from_raw(
		tag: sys::wimlib_progress_msg,
		payload: *mut sys::wimlib_progress_info,
	) -> Option<Self> {
		let new = match tag {
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_IMAGE_BEGIN => {
				let payload = unsafe { ExtractMsg::from_raw((*payload).extract)? };
				Self::ExtractImageBegin(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_TREE_BEGIN => {
				let payload = unsafe { ExtractMsg::from_raw((*payload).extract)? };
				Self::ExtractTreeBegin(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_FILE_STRUCTURE => {
				let payload =
					unsafe { ExtractMsg::with_extract_fs_or_metadata_extras((*payload).extract)? };

				Self::ExtractFileStructure(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_STREAMS => {
				let payload = unsafe { ExtractMsg::from_raw((*payload).extract)? };
				Self::ExtractStreams(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_METADATA => {
				let payload =
					unsafe { ExtractMsg::with_extract_fs_or_metadata_extras((*payload).extract)? };

				Self::ExtractFileStructure(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_IMAGE_END => {
				let payload = unsafe { ExtractMsg::from_raw((*payload).extract)? };
				Self::ExtractImageEnd(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_EXTRACT_TREE_END => {
				let payload = unsafe { ExtractMsg::from_raw((*payload).extract)? };
				Self::ExtractTreeEnd(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_SCAN_BEGIN => {
				let payload = unsafe { ScanMsg::from_raw((*payload).scan) };
				Self::ScanBegin(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_SCAN_DENTRY => {
				let (raw, scan);
				unsafe {
					raw = (*payload).scan;
					scan = ScanMsg::from_raw(raw);
				}

				Self::ScanDentry {
					scan,
					cur_path: unsafe { TStr::from_ptr(raw.cur_path) },
					status: ScanDentryStatus::from_raw(raw.status)?,
				}
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_SCAN_END => {
				let payload = unsafe { ScanMsg::from_raw((*payload).scan) };
				Self::ScanEnd(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_WRITE_STREAMS => {
				let raw = unsafe { (*payload).write_streams };
				Self::WriteStreams {
					total_bytes: raw.total_bytes,
					total_streams: raw.total_streams,
					completed_bytes: raw.completed_bytes,
					completed_streams: raw.completed_streams,
					num_threads: raw.num_threads,
					compression_type: CompressionType::from_raw(raw.compression_type),
					total_parts: raw.total_parts,
					completed_parts: raw.completed_parts,
					completed_compression_bytes: raw.completed_compressed_bytes,
				}
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_WRITE_METADATA_BEGIN => {
				Self::WriteMetadataBegin
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_WRITE_METADATA_END => {
				Self::WriteMetadataEnd
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_RENAME => unsafe {
				let payload = (*payload).rename;
				Self::Rename {
					from: TStr::from_ptr(payload.from),
					to: TStr::from_ptr(payload.to),
				}
			},
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_VERIFY_INTEGRITY => unsafe {
				let raw = (*payload).integrity;
				let filename = TStr::from_ptr(raw.filename);
				let calc = IntegrityMsg::from_raw(raw);

				Self::VerifyIntegrity { calc, filename }
			},
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_CALC_INTEGRITY => {
				let raw = unsafe { (*payload).integrity };
				let payload = IntegrityMsg::from_raw(raw);
				Self::CalcIntegrity(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_SPLIT_BEGIN_PART => {
				let raw = unsafe { (*payload).split };
				Self::SplitBeginPart(SplitMsg::from_raw(raw))
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_SPLIT_END_PART => {
				let raw = unsafe { (*payload).split };
				Self::SplitEndPart(SplitMsg::from_raw(raw))
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_UPDATE_BEGIN_COMMAND => {
				let raw = unsafe { (*payload).update };
				let payload = UpdateMsg::from_raw(raw);
				Self::UpdateBeginCommand(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_UPDATE_END_COMMAND => {
				let raw = unsafe { (*payload).update };
				let payload = UpdateMsg::from_raw(raw);
				Self::UpdateEndCommand(payload)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_REPLACE_FILE_IN_WIM => {
				let path = unsafe {
					let raw = (*payload).replace.path_in_wim;
					TStr::from_ptr(raw)
				};

				Self::ReplaceFileInWim { path }
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_WIMBOOT_EXCLUDE => unsafe {
				let raw = (*payload).wimboot_exclude;
				let path_in_wim = TStr::from_ptr(raw.path_in_wim);
				let extraction_path = TStr::from_ptr(raw.extraction_path);
				Self::WimbootExclude {
					path_in_wim,
					extraction_path,
				}
			},
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_UNMOUNT_BEGIN => {
				let (raw, mountpoint, mounted_wim);
				unsafe {
					raw = (*payload).unmount;
					mountpoint = TStr::from_ptr(raw.mountpoint);
					mounted_wim = TStr::from_ptr(raw.mounted_wim);
				}

				let mounted_image = NonZero::new(raw.mounted_image)?;

				Self::UnmountBegin {
					mountpoint,
					mounted_wim,
					mounted_image,
					mount_flags: raw.mount_flags,
					unmount_flags: raw.unmount_flags,
				}
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_DONE_WITH_FILE => {
				let path_to_file =
					unsafe { TStr::from_ptr((*payload).done_with_file.path_to_file) };

				Self::DoneWithFile { path_to_file }
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_BEGIN_VERIFY_IMAGE => {
				let raw = unsafe { VerifyImageMsg::from_raw((*payload).verify_image) };
				Self::BeginVerifyImage(raw)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_END_VERIFY_IMAGE => {
				let raw = unsafe { VerifyImageMsg::from_raw((*payload).verify_image) };
				Self::BeginVerifyImage(raw)
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_VERIFY_STREAMS => {
				let (raw, wimfile);
				unsafe {
					raw = (*payload).verify_streams;
					wimfile = TStr::from_ptr(raw.wimfile);
				}

				Self::VerifyStreams {
					wimfile,
					total_streams: raw.total_streams,
					total_bytes: raw.total_bytes,
					completed_streams: raw.completed_streams,
					completed_bytes: raw.completed_bytes,
				}
			}
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_TEST_FILE_EXCLUSION => unsafe {
				let raw = (*payload).test_file_exclusion;
				let path = TStr::from_ptr(raw.path);
				let will_exclude = &mut (*payload).test_file_exclusion.will_exclude;

				Self::TestFileExclusion { path, will_exclude }
			},
			sys::wimlib_progress_msg_WIMLIB_PROGRESS_MSG_HANDLE_ERROR => unsafe {
				let raw = (*payload).handle_error;
				let path = TStr::from_ptr_optional(raw.path);
				let error_code = raw.error_code;
				let will_ignore = &mut (*payload).handle_error.will_ignore;

				Self::HandleError {
					path,
					error_code,
					will_ignore,
				}
			},
			_ => return None,
		};

		Some(new)
	}
}

impl ExtractMsg<'_> {
	unsafe fn from_raw(
		payload: sys::wimlib_progress_info_wimlib_progress_info_extract,
	) -> Option<Self> {
		let (wimfile_name, image_name, target);
		unsafe {
			wimfile_name = TStr::from_ptr_optional(payload.wimfile_name);
			image_name = TStr::from_ptr(payload.image_name);
			target = TStr::from_ptr(payload.target);
		}

		let image = NonZero::new(payload.image)?;

		Some(Self {
			image,
			extract_flags: ExtractFlags::from_bits(payload.extract_flags as _)?,
			wimfile_name,
			image_name,
			target,
			total_bytes: payload.total_bytes,
			completed_bytes: payload.completed_bytes,
			extras: (),
		})
	}
}

impl ExtractMsg<'_, ExtractFsOrMetadataExtras> {
	unsafe fn with_extract_fs_or_metadata_extras(
		payload: sys::wimlib_progress_info_wimlib_progress_info_extract,
	) -> Option<Self> {
		let payload_0 = unsafe { ExtractMsg::from_raw(payload)? };

		let extras = ExtractFsOrMetadataExtras {
			current_file_count: payload.current_file_count,
			end_file_count: payload.end_file_count,
		};

		Some(payload_0.with_extras(extras))
	}
}

impl<'a, Extras> ExtractMsg<'a, Extras> {
	fn with_extras<NewExtras>(self, extras: NewExtras) -> ExtractMsg<'a, NewExtras> {
		ExtractMsg {
			image: self.image,
			extract_flags: self.extract_flags,
			wimfile_name: self.wimfile_name,
			image_name: self.image_name,
			target: self.target,
			total_bytes: self.total_bytes,
			completed_bytes: self.completed_bytes,
			extras,
		}
	}
}

impl ScanMsg<'_> {
	unsafe fn from_raw(payload: sys::wimlib_progress_info_wimlib_progress_info_scan) -> Self {
		let (source, target_path);
		unsafe {
			source = TStr::from_ptr(payload.source);
			target_path = TStr::from_ptr(payload.__bindgen_anon_1.wim_target_path);
		};

		Self {
			source,
			target_path,
			num_dirs_scanned: payload.num_dirs_scanned,
			num_nondirs_scanned: payload.num_nondirs_scanned,
			num_bytes_scanned: payload.num_bytes_scanned,
		}
	}
}

impl ScanDentryStatus {
	const fn from_raw(val: u32) -> Option<Self> {
		let new = match val {
			sys::wimlib_progress_info_wimlib_progress_info_scan_WIMLIB_SCAN_DENTRY_OK => Self::Ok,
			sys::wimlib_progress_info_wimlib_progress_info_scan_WIMLIB_SCAN_DENTRY_EXCLUDED => Self::Exluded,
			sys::wimlib_progress_info_wimlib_progress_info_scan_WIMLIB_SCAN_DENTRY_UNSUPPORTED => Self::Unsupported,
			sys::wimlib_progress_info_wimlib_progress_info_scan_WIMLIB_SCAN_DENTRY_FIXED_SYMLINK => Self::FixedSymlink,
			sys::wimlib_progress_info_wimlib_progress_info_scan_WIMLIB_SCAN_DENTRY_NOT_FIXED_SYMLINK => Self::NotFixedSymlink,
			_ => return None,
		};

		Some(new)
	}
}

impl IntegrityMsg {
	const fn from_raw(payload: sys::wimlib_progress_info_wimlib_progress_info_integrity) -> Self {
		Self {
			total_bytes: payload.total_bytes,
			completed_bytes: payload.completed_bytes,
			total_chunks: payload.total_chunks,
			completed_chunks: payload.completed_chunks,
			chunk_size: payload.chunk_size,
		}
	}
}

impl VerifyImageMsg<'_> {
	unsafe fn from_raw(
		payload: sys::wimlib_progress_info_wimlib_progress_info_verify_image,
	) -> Self {
		let wimfile = unsafe { TStr::from_ptr(payload.wimfile) };
		Self {
			wimfile,
			total_images: payload.total_images,
			current_image: payload.current_image,
		}
	}
}
