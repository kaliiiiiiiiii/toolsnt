//! Create and handle non-standalone WIMs, such as split and delta WIMs
//!
//! A [`Wim`] backed by an on-disk file normally represents a fully standalone
//! WIM archive. However, WIM archives can also be arranged in non-standalone
//! ways, such as a set of on-disk files that together form a single »split WIM«
//! or »delta WIM«. Such arrangements are fully supported by wimlib. However, as
//! a result, in such cases a [`Wim`] created from one of these on-disk files
//! initially only partially represents the full WIM and needs to, in effect, be
//! logically combined with other [`Wim`]'s before performing certain
//! operations, such as extracting files with [`crate::Image::extract`] or
//! [`crate::Image::extract_from_paths`]. This is done by calling
//! [`Wim::reference_resource_files`] or [`Wim::reference_resource_files`].
//! Note: if you fail to do so, you may see the error code
//! [`Error::ResourceNotFound`]; this just indicates that data is not available
//! because the appropriate WIM files have not yet been referenced.

use {
	crate::{
		error::result_from_raw,
		progress::{trampoline, ProgressCallback},
		string::{TStr, ThinTStr},
		sys, Error, OpenFlags, Wim, WimLib, WriteFlags,
	},
	std::marker::PhantomData,
};

impl WimLib {
	/// Join a split WIM into a stand-alone (one-part) WIM
	///
	/// # Parameters
	/// - `split_wims`: Array of strings that gives the filenames of all parts
	///   of the split WIM. No specific order is required, but all parts must be
	///   included with no duplicates
	/// - `split_wims_open_flags`: Open flags for the split WIM parts (eg.
	///   [`OpenFlags::CHECK_INTEGRITY`])
	/// - `wim_write_flags`: Write flags for writing the joined WIM
	/// - `output_path`: Output path of the joined WIM
	///
	/// # Error values
	/// - [`Error::SplitInvalid`]: The split WIMs do not form a valid WIM
	///   because they do not include all the parts of the original WIM, there
	///   are duplicate parts, or not all the parts have the same GUID and
	///   compression type
	/// - [`Error::InvalidParam`]: The length of `split_wims` > [`u32::MAX`]
	///
	/// # Note
	/// wimlib is generalized enough that this function is not actually needed
	/// to join a split WIM; instead, you could open the first part of the split
	/// WIM, then reference the other parts with
	/// wimlib_reference_resource_files(), then write the joined WIM using
	/// [`crate::Image::write`]. However, [`Self::join`] provides an
	/// easy-to-use wrapper around this that has some advantages (e.g. extra
	/// sanity checks).
	#[doc(alias = "wimlib_join")]
	pub fn join(
		&self,
		split_wims: &[ThinTStr],
		output_path: &TStr,
		split_open_flags: OpenFlags,
		joined_write_flags: WriteFlags,
	) -> Result<(), Error> {
		let split_wims_count = cast_length_to_u32(split_wims.len())?;

		result_from_raw(unsafe {
			sys::wimlib_join(
				split_wims.as_ptr().cast(),
				split_wims_count,
				output_path.as_ptr(),
				split_open_flags.bits(),
				joined_write_flags.bits(),
			)
		})
	}

	/// Sane as [`Self::join`], but allows specifying a progress function
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::WriteStreams`] while writing the
	///   joined WIM
	/// - [`crate::progress::ProgressMsg::VerifyIntegrity`] messages if
	///   [`crate::OpenFlags::CHECK_INTEGRITY`] specified in `split_open_flags`
	///   when each split WIM part is opened
	#[doc(alias = "wimlib_join_with_progress")]
	pub fn join_with_progress_callback(
		&self,
		split_wims: &[ThinTStr],
		output_path: &TStr,
		split_open_flags: OpenFlags,
		joined_write_flags: WriteFlags,
		progress_callback: &mut ProgressCallback,
	) -> Result<(), Error> {
		let split_wims_count = cast_length_to_u32(split_wims.len())?;
		let progress_thin: *mut *mut ProgressCallback = &mut (progress_callback as *mut _);

		result_from_raw(unsafe {
			sys::wimlib_join_with_progress(
				split_wims.as_ptr().cast(),
				split_wims_count,
				output_path.as_ptr(),
				split_open_flags.bits(),
				joined_write_flags.bits(),
				Some(trampoline),
				progress_thin.cast(),
			)
		})
	}
}

impl Wim {
	/// Reference file data from other WIM files or split WIM parts
	///
	/// This function can be used on WIMs that are not standalone, such as split
	/// or »delta« WIMs, to load additional file data before calling a function
	/// such as [`crate::Image::extract`] that requires the file data to be
	/// present.
	///
	/// # Parameters
	/// - `resource_wimfiles_or_globs`: Array of paths to WIM files and/or split
	///   WIM parts to reference. Alternatively, when
	///   [`ReferenceFlags::GLOB_ENABLE`] is specified in `ref_flags`, hese are
	///   treated as globs rather than literal paths. That is, using this
	///   function you can specify zero or more globs, each of which expands to
	///   one or more literal paths
	/// - `ref_flags`: Extra flags for referencing resource files
	/// - `open_flags`: Extra flags for opening resource files
	///
	/// # Error values
	/// - [`Error::GlobHadNoMatches`]: One of the specified globs did not match
	///   any paths
	/// - [`Error::Read`]: I/O or permissions error while processing a file glob
	/// - Refer to [`WimLib::open_wim`]
	#[doc(alias = "wimlib_reference_resource_files")]
	pub fn reference_resource_files(
		&self,
		resource_wimfiles_or_globs: &[ThinTStr],
		ref_flags: ReferenceFlags,
		open_flags: OpenFlags,
	) -> Result<(), Error> {
		let resource_wimfiles_or_globs_count =
			cast_length_to_u32(resource_wimfiles_or_globs.len())?;

		result_from_raw(unsafe {
			sys::wimlib_reference_resource_files(
				self.wimstruct,
				resource_wimfiles_or_globs.as_ptr().cast(),
				resource_wimfiles_or_globs_count,
				ref_flags.bits(),
				open_flags.bits(),
			)
		})
	}

	/// Similar to [`Self::reference_resource_files`], but operates at a lower
	/// level where the caller must open the [`Wim`] for each referenced file
	/// itself
	///
	/// # Error values
	/// Refer to [`Self::reference_resource_files`]
	#[doc(alias = "wimlib_reference_resources")]
	pub fn reference_resources(
		&self,
		resource_wims: &[ResourceWimRef],
		ref_flags: ReferenceFlags,
	) -> Result<(), Error> {
		let resource_wims_count = cast_length_to_u32(resource_wims.len())?;

		result_from_raw(unsafe {
			sys::wimlib_reference_resources(
				self.wimstruct,
				resource_wims.as_ptr().cast_mut().cast(),
				resource_wims_count,
				ref_flags.bits(),
			)
		})
	}

	/// Split a WIM into multiple parts
	///
	/// # Parameters
	/// `split_name`: Name of the split WIM file to create. This will be the
	/// name of the first part. The other parts will, by default, have the same
	/// name with 2, 3, 4, ..., etc. appended before the suffix. However, the
	/// exact names can be customized using the progress function. `part_size`:
	/// The maximum size part, in bytes. Unfortunately, it is not guaranteed
	/// that this will really be the maximum size per part, because some file
	/// resources in the WIM may be larger than this size, and the WIM file
	/// format provides no way to split up file resources among multiple WIMs.
	/// `write_flags`: Extra flags for writing the split WIMs
	///
	/// # Error values
	/// - Refer to [`crate::Image::write`]
	/// - [`Error::InvalidParam`]: `split_name` was an empty string, or
	///   `part_size` was 0
	/// - [`Error::Unsupported`]: The WIM contains solid resources. Splitting a
	///   WIM containing solid resources is not supported.
	///
	/// # Progress messages
	/// - [`crate::progress::ProgressMsg::SplitBeginPart`] and
	///   [`crate::progress::ProgressMsg::SplitEndPart`] for each part written
	/// - [`crate::progress::ProgressMsg::WriteStreams`] while writing each part
	///
	/// These report the progress of the current part only
	pub fn split(
		&self,
		split_name: &TStr,
		size: u64,
		write_flags: WriteFlags,
	) -> Result<(), Error> {
		result_from_raw(unsafe {
			sys::wimlib_split(
				self.wimstruct,
				split_name.as_ptr(),
				size,
				write_flags.bits(),
			)
		})
	}
}

/// Wim resource reference
///
/// References its underlying WIM structure.
#[repr(transparent)]
pub struct ResourceWimRef<'a> {
	inner: *mut sys::WIMStruct,
	_borrow: PhantomData<&'a TStr>,
}

impl<'a> From<&'a Wim> for ResourceWimRef<'a> {
	fn from(value: &'a Wim) -> Self {
		Self::new(value)
	}
}

impl<'a> ResourceWimRef<'a> {
	/// Create a new [`ResourceWimRef`]
	pub fn new(wim: &'a Wim) -> Self {
		Self {
			inner: wim.wimstruct,
			_borrow: PhantomData,
		}
	}

	/// Create a new array of [`ResourceWimRef`] from a [`Wim`]
	pub fn array<const N: usize>(paths: [&'a Wim; N]) -> [Self; N] {
		paths.map(Self::new)
	}
}

fn cast_length_to_u32(value: usize) -> Result<u32, Error> {
	u32::try_from(value).map_err(|_| Error::InvalidParam)
}

bitflags::bitflags! {
	/// Resource reference flags
	pub struct ReferenceFlags: std::ffi::c_int {
		/// For [`Wim::reference_resource_files`], enable shell-style filename
		/// globbing
		///
		/// Ignored by [`Wim::reference_resources`]
		const GLOB_ENABLE         = sys::WIMLIB_REF_FLAG_GLOB_ENABLE         as _;
		/// For [`Wim::reference_resource_files`], issue an error
		/// ([`Error::GlobHadNoMatches`]) if a glob did not match any files
		///
		/// The default behavior without this flag is to issue no error at that
		/// point, but then attempt to open the glob as a literal path, which of
		/// course will fail anyway if no file exists at that path. No effect if
		/// [`Self::GLOB_ENABLE`] is not also specified. Ignored by
		/// Ignored by [`Wim::reference_resources`].
		const GLOB_ERR_ON_NOMATCH = sys::WIMLIB_REF_FLAG_GLOB_ERR_ON_NOMATCH as _;
	}
}
