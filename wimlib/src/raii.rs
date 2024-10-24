//! Resource management

use {
	crate::{error::result_from_raw, sys},
	std::sync::{Mutex, MutexGuard},
};

/// Lock counter ignoring poison (cases of panic are rare and should be
/// harmless)
fn lock_counter<'a>() -> MutexGuard<'a, usize> {
	static LOCKED_COUNTER: Mutex<usize> = Mutex::new(0);
	match LOCKED_COUNTER.lock() {
		Ok(lock) => lock,
		Err(poison) => {
			LOCKED_COUNTER.clear_poison();
			poison.into_inner()
		}
	}
}

/// An zero-sized type which guards the resources by counting global refernece
/// counter
pub struct WimLib {
	_private: (),
}

impl Clone for WimLib {
	fn clone(&self) -> Self {
		*lock_counter() += 1;
		Self { _private: () }
	}
}

impl Drop for WimLib {
	fn drop(&mut self) {
		let mut counter_lock = lock_counter();
		*counter_lock -= 1;

		if *counter_lock == 0 {
			unsafe { sys::wimlib_global_cleanup() };
		}
	}
}

impl Default for WimLib {
	/// Obtain wimlib handle. If not initialised, initialise with
	/// [`InitFlags::empty()`] flags
	fn default() -> Self {
		let mut counter_lock = lock_counter();
		if *counter_lock == 0 {
			let stat = unsafe { sys::wimlib_global_init(InitFlags::empty().bits()) };
			assert_eq!(
				stat, 0,
				"initialising wimlib with empty flags should always return zero"
			)
		}

		*counter_lock += 1;
		Self { _private: () }
	}
}

impl WimLib {
	/// Initialise wimlib
	///
	/// If already initialised, return [`TryInitError::AlreadyInitialised`].
	///
	/// If [`InitFlags::STRICT_CAPTURE_PRIVILEGES`] or
	/// [`InitFlags::STRICT_APPLY_PRIVILEGES`] set and corresponding privileges
	/// could not be acquired, returns [`TryInitError::Wimlib`].
	pub fn try_init(flags: InitFlags) -> Result<Self, TryInitError> {
		let mut counter_lock = lock_counter();
		if *counter_lock == 0 {
			return Err(TryInitError::AlreadyInitialised);
		}

		let stat = unsafe { sys::wimlib_global_init(flags.bits()) };
		result_from_raw(stat).map_err(TryInitError::Wimlib)?;
		*counter_lock += 1;

		Ok(Self { _private: () })
	}
}

/// Error returned when tried to initialise WimLib
pub enum TryInitError {
	/// wimlib is already initialised – you need to drop all [`WimLib`]
	/// instances and start over to change flags or obtain a handle using
	/// [`WimLib::default`]
	AlreadyInitialised,
	/// wimlib initialisation error
	Wimlib(crate::Error),
}

bitflags::bitflags! {
	/// Flags for initialising the library using [`WimLib::try_init`]
	#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
	pub struct InitFlags: std::ffi::c_int {
		#[cfg(any(windows, doc))]
		/// Do not attempt to acquire additional privileges
		/// (currently SeBackupPrivilege, SeRestorePrivilege, SeSecurityPrivilege,
		/// SeTakeOwnershipPrivilege, and SeManageVolumePrivilege) when initializing the library.
		const DONT_ACQUIRE_PRIVILEGES   = sys::WIMLIB_INIT_FLAG_DONT_ACQUIRE_PRIVILEGES   as _;

		#[cfg(any(windows, doc))]
		/// If [`Self::DONT_ACQUIRE_PRIVIELGES`] not specified,
		/// return [`Error::InsufficientPrivileges`] if privileges that may be needed
		/// to read all possible data and metadata for a capture operation could not be acquired.
		const STRICT_CAPTURE_PRIVILEGES = sys::WIMLIB_INIT_FLAG_STRICT_CAPTURE_PRIVILEGES as _;

		#[cfg(any(windows, doc))]
		/// If [`Self::DONT_ACQUIRE_PRIVIELGES`] not specified,
		/// return [`Error::InsufficientPrivileges`] if privileges that may be needed
		/// to restore all possible data and metadata for an apply operation could not be acquired.
		const STRICT_APPLY_PRIVILEGES   = sys::WIMLIB_INIT_FLAG_STRICT_APPLY_PRIVILEGES   as _;

		/// Default to interpreting WIM paths case sensitively (default on UNIX-like systems)
		const DEFAULT_CASE_SENSITIVE    = sys::WIMLIB_INIT_FLAG_DEFAULT_CASE_SENSITIVE    as _;

		/// Default to interpreting WIM paths case insensitively (default on Windows)
		const DEFAULT_CASE_INSENSITIVE  = sys::WIMLIB_INIT_FLAG_DEFAULT_CASE_INSENSITIVE  as _;
	}
}
