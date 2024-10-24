//! Open an existing WIM file as a [`Wim`], or create a new [`Wim`] which can be
//! used to create a new WIM file. Also functions to set your progress
//! functions or verify a WIM.

use {
	super::{CompressionType, SavedProgressCallback, Wim},
	crate::{
		error::result_from_raw,
		progress::{trampoline, ProgressCallback, ProgressMsg, ProgressStatus},
		string::TStr,
		sys, Error, WimLib,
	},
	std::{
		mem::MaybeUninit,
		ptr::{addr_of_mut, null_mut, NonNull},
	},
};

impl Wim {
	/// Register a progress function
	pub fn register_progress_callback(
		&mut self,
		progress_callback: impl FnMut(&mut ProgressMsg) -> ProgressStatus + 'static,
	) {
		let ptr = thin_ptr_box_progress_callback(progress_callback);
		self.progress_callback = Some(SavedProgressCallback(ptr));
		unsafe {
			sys::wimlib_register_progress_function(
				self.wimstruct,
				Some(trampoline),
				ptr.as_ptr().cast(),
			)
		};
	}

	/// Unregister a progress function
	pub fn clear_progress_function(&mut self) {
		self.progress_callback = None;
		unsafe { sys::wimlib_register_progress_function(self.wimstruct, None, null_mut()) };
	}

	/// Perform verification checks on a WIM file
	///
	/// This function is intended for safety checking and/or debugging. If used
	/// on a well-formed WIM file, it should always succeed.
	///
	/// # Error value
	/// - [`Error::Decompression`]: The WIM file contains invalid compressed
	///   data
	/// - [`Error::InvalidMetadataResource`]: The metadata resource for an image
	///   is invalid
	/// - [`Error::InvalidResourceHash`]: File data stored in the WIM file is
	///   corrupt
	/// - [`Error::ResourceNotFound`]: The data for a file in an image could not
	///   be found. See [Creating and handling non-standalone
	///   WIMs](crate::wim::non_standalone).
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::BeginVerifyImage`]
	/// - [`crate::progress::ProgressMsg::EndVerifyImage`]
	/// - [`crate::progress::ProgressMsg::VerifyStreams`]
	pub fn verify(&self, verify_flags: VerifyFlags) -> Result<(), Error> {
		result_from_raw(unsafe { sys::wimlib_verify_wim(self.wimstruct, verify_flags.bits()) })
	}
}

impl WimLib {
	/// Create a [`Wim`] which initially has no images and is not backed by an
	/// on-disk file
	///
	/// # Error values
	/// - [`Error::InvalidCompressionType`]: `compression_type` was not a
	///   supported compression type
	/// - [`Error::Nomem`]: Insufficient memory to allocate a new
	///   [`sys::WIMStruct`] backing [`Wim`]
	#[doc(alias = "wimlib_create_new_wim")]
	pub fn create_new_wim(&self, compression_type: CompressionType) -> Result<Wim, Error> {
		let mut wimstruct = null_mut();

		result_from_raw(unsafe {
			sys::wimlib_create_new_wim(compression_type as _, &mut wimstruct)
		})?;

		Ok(Wim {
			wimstruct,
			progress_callback: None,
			_wimlib: self.clone(),
		})
	}

	/// Open an image file
	///
	/// # Error values
	/// - [`Error::ImageCount`]: The number of metadata resources found in the
	///   WIM did not match the image count specified in the image header, or
	///   the number of \<IMAGE\> elements in the XML data of the image did not
	///   match the image count specified in the WIM header
	/// - [`Error::Integrity`]: [`OpenFlags::CHECK_INTEGRITY`] was specified in
	///   open_flags, and the WIM file failed the integrity check
	/// - [`Error::InvalidChunkSize`]: The library did not recognize the
	///   compression chunk size of the image as valid for its compression type
	/// - [`Error::InvalidHeader`]: The header of the WIM was otherwise invalid
	/// - [`Error::InvalidIntegrityTable`]: [`OpenFlags::CHECK_INTEGRITY`] was
	///   specified in open_flags and the WIM contained an integrity table, but
	///   the integrity table was invalid
	/// - [`Error::InvalidLookupTableEntry`]: The lookup table of the WIM was
	///   invalid
	/// - [`Error::InvalidParam`]: `path` was empty
	/// - [`Error::IsSplitWim`]: The WIM was a split WIM and
	///   [`OpenFlags::ERROR_IF_SPLIT`] was specified in open_flags
	/// - [`Error::NotAWimFile`]: The file did not begin with the magic
	///   characters that identify a WIM file
	/// - [`Error::Open`]: Failed to open the WIM file for reading. Some
	///   possible reasons: the WIM file does not exist, or the calling process
	///   does not have permission to open it
	/// - [`Error::Read`]: Failed to read data from the WIM file
	/// - [`Error::UnexpectedEndOfFile`]: Unexpected end-of-file while reading
	///   data from the WIM file
	/// - [`Error::UnknownVersion`]: The WIM version number was not recognized.
	///   (May be a pre-Vista WIM.)
	/// - [`Error::WimIsEncrypted`]: The WIM cannot be opened because it
	///   contains encrypted segments. (It may be a Windows 8 »ESD« file.)
	/// - [`Error::WimIsIncomplete`]: The WIM file is not complete (e.g. the
	///   program which wrote it was terminated before it finished)
	/// - [`Error::WimIsReadonly`]: [`OpenFlags::WRITE_ACCESS`] was specified
	///   but the WIM file was considered read-only because of any of the
	///   reasons mentioned in the documentation for the
	///   [`OpenFlags::WRITE_ACCESS`] flag
	/// - [`Error::Xml`]: The XML data of the WIM was invalid
	#[doc(alias = "wimlib_open_wim")]
	pub fn open_wim(&self, path: &TStr, open_flags: OpenFlags) -> Result<Wim, Error> {
		// [`Error::InvalidParam`] can also occur when reference to `wim` below was
		// null.

		let mut wimstruct = null_mut();

		result_from_raw(unsafe {
			sys::wimlib_open_wim(path.as_ptr(), open_flags.bits(), &mut wimstruct)
		})?;

		Ok(Wim {
			wimstruct,
			progress_callback: None,
			_wimlib: self.clone(),
		})
	}

	/// Same as [`Self::open_wim`], but allows specifying a progress callback
	/// and progress context
	///
	/// If [`OpenFlags::CHECK_INTEGRITY`] is specified in `open_flags`, then the
	/// progress function will receive [`ProgressMsg::VerifyIntegrity`] messages
	/// when checking the WIM file's integrity
	///
	/// # Error values
	/// - Same as for [`Self::open_wim`]
	#[doc(alias = "wimlib_open_wim_with_progress")]
	pub fn open_wim_with_progress_callback(
		&self,
		path: &TStr,
		open_flags: OpenFlags,
		progress_callback: impl FnMut(&mut ProgressMsg) -> ProgressStatus + 'static,
	) -> Result<Wim, Error> {
		unsafe {
			let mut new_uninit = MaybeUninit::<Wim>::uninit();
			let new_ptr = new_uninit.as_mut_ptr();

			let nonnull_callback_ptr = thin_ptr_box_progress_callback(progress_callback);

			// Write a progress callback so we can reference it from within this
			let progress_callback_ptr = addr_of_mut!((*new_ptr).progress_callback);
			progress_callback_ptr.write(Some(SavedProgressCallback(nonnull_callback_ptr)));

			// This also writes the `wim` field
			result_from_raw(sys::wimlib_open_wim_with_progress(
				path.as_ptr(),
				open_flags.bits(),
				addr_of_mut!((*new_ptr).wimstruct),
				Some(trampoline),
				nonnull_callback_ptr.as_ptr().cast(),
			))?;

			// And finally write the reference counter thing
			addr_of_mut!((*new_ptr)._wimlib).write(self.clone());
			Ok(new_uninit.assume_init())
		}
	}
}

bitflags::bitflags! {
	/// Flags related to [`WimLib::open_wim`]
	pub struct OpenFlags: std::ffi::c_int {
		/// Verify the WIM contents against the WIM's integrity table, if present
		const CHECK_INTEGRITY = sys::WIMLIB_OPEN_FLAG_CHECK_INTEGRITY as _;
		/// Issue an error ([`Error::IsSplitWim`]) if the WIM is part of a split WIM
		const ERROR_IF_SPLIT  = sys::WIMLIB_OPEN_FLAG_ERROR_IF_SPLIT  as _;
		/// Check if the WIM is writable and issue an error ([`Error::WimIsReadonly`])
		/// if it is not.
		const WRITE_ACCESS    = sys::WIMLIB_OPEN_FLAG_WRITE_ACCESS    as _;
	}

	/// Flags related to [`Wim::verify`]; currently only reserved
	pub struct VerifyFlags: std::ffi::c_int {}
}

fn thin_ptr_box_progress_callback<C>(
	callback: C,
) -> NonNull<*mut (dyn FnMut(&mut ProgressMsg) -> ProgressStatus + 'static)>
where
	C: FnMut(&mut ProgressMsg) -> ProgressStatus + 'static,
{
	let boxed_callback_ptr: Box<*mut ProgressCallback> =
		Box::new(Box::into_raw(Box::new(callback)));

	unsafe { NonNull::new_unchecked(Box::into_raw(boxed_callback_ptr)) }
}
