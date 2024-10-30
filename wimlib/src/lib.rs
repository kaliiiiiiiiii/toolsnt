//! # wimlib
//!
//! These are wimlib Rust bindings. wimlib is a C library for creating,
//! modifying, extracting and mounting files in the Windows Imaging Format (WIM
//! files). It provides a free and corss-platform alternative to Microsoft's
//! WIMGAPI, ImageX and DISM.
//!
//! C library link: <https://wimlib.net>
//!
//! # Basic WIM handling concepts
//! Wimlib bindings wrap up a WIM file in a opaque [`Wim`] type. There are two
//! ways to create its instance
//! 1. [`WimLib::open_wim`] opens an on-disk WIM file and creates a [`Wim`] for
//!    it
//! 2. [`WimLib::create_new_wim`] creates a new [`Wim`] that initially contains
//!    no images and does not yet have a backing on-disk file
//!
//! [`Wim`] contains zero or more independent directory trees called images.
//! Images may be extracted, added, deleted, exported, and updated using various
//! API functions. You can select these images using [`Wim::select_image`] and
//! [`Wim::select_all_images`] as most of methods using them are under structure
//! [`Image`].
//!
//! Changes made to a WIM represented by a [`Wim`] have no persistent effect
//! until the WIM is actually written to an on-disk file. This can be done using
//! [`Image::write`], but if the WIM was originally opened using
//! [`WimLib::open_wim`], then [`Wim::overwrite`] can be used instead.
//!
//! wimlib's API is designed to let you combine functions to accomplish tasks in
//! a flexible way. Here are some example sequences of function calls:
//!
//! Apply an image from a WIM file, similar to the command-line program
//! **wimapply**
//! 1. [`WimLib::open_wim`]
//! 2. [`Image::extract`]
//!
//! Capture an image into a new WIM file, similar to **wimcapture**:
//! 1. [`WimLib::create_new_wim`]
//! 2. [`Wim::add_image`]
//! 3. [`Image::write`]
//!
//! Append an image to an existing WIM file, similar to **wimappend**:
//! 1. [`WimLib::open_wim`]
//! 2. [`Wim::add_image`]
//! 3. [`Wim::overwrite`]
//!
//! Delete an image from an existing WIM file, similar to **wimdelete**:
//! 1. [`WimLib::open_wim`]
//! 2. [`Image::delete_self`]
//! 3. [`Wim::overwrite`]
//!
//! Export an image from one WIM file to another, similar to **wimexport**:
//! 1. [`WimLib::open_wim`] (on source)
//! 2. [`WimLib::open_wim`] (on destination)
//! 3. [`Image::export`]
//! 4. [`Wim::overwrite`] (on destination)
//!
//! The API also lets you do things the command-line tools don't directly allow.
//! For example, you could make multiple changes to a WIM before efficiently
//! committing the changes with just one call to [`Wim::overwrite`]. Perhaps you
//! want to both delete an image and add a new one; or perhaps you want to
//! customize an image with [`Wim::overwrite`] after adding it. All these use
//! cases are supported by the API.
//!
//! You can also visit documentation of modules in [wim] module for detailed
//! documentation of certain scopes.
//!
//! # Additional information and features
//! ## Mounting WIM images
//! See [Mounting WIM images](crate::wim::mount).
//!
//! ## Progress Messages
//! See [progress::ProgressMsg].
//!
//! ## Non-standalone WIMs
//! See [Creating and handling non-standalone WIMSs](wim::non_standalone).
//! ## Pipable WIMs
//! wimlib supports a special »pipable« WIM format which unfortunately is not
//! compatible with Microsoft's software. To create a pipable WIM, call
//! [`Image::write`], [`Image::write_to_file`], or [`Wim::overwrite`] with
//! [`WriteFlags::PIPABLE`] specified. Pipable WIMs are pipable in both
//! directions, so [`Image::write_to_file`] can be used to write a pipable WIM
//! to a pipe, and [`WimLib::extract_image_from_pipe`] can be used to apply an
//! image from a pipable WIM. wimlib can also transparently open and operate on
//! pipable WIM s using a seekable file descriptor using the regular function
//! calls (e.g. [`WimLib::open_wim`], [`Image::extract`]).
//!
//! See the documentation for the `–pipable` flag of `wimcapture` for more
//! information about pipable WIMs.
//!
//! ## Custom allocation functions
//! WimLib allows settings custom allocator functions. I would really like to
//! make [`wimlib`] bindings support this feature, especially by automatically
//! hooking that to users global allocator, but…
//! 1. It uses C-style allocation APIs, which are very basic compared to Rusts
//!    and inherently incompatible. Rust's [allocation
//!    APIs][`std::alloc::GlobalAlloc`] require knowing
//!    [`Layout`][`std::alloc::Layout`]. which includes alignment. `malloc`
//!    requires only size. For dallocation memory, C's API is even more basic.
//!    Just provide a pointer. No size, no alignment. This can be bypassed by
//!    having some fixed alignment and allocating a bit more to store the size
//!    information and return shifted pointer. Tried this and it lead me to…
//!
//! 2. `wimlib` calls `libc`'s `realpath`. That one allocates using `libc`'s
//!    allocation facilities (`malloc`) but `wimlib` then deallocates the memory
//!    by user provided deallocation function. And as my deallocation trick
//!    shifted pointer back to the header and tried to read it, it made
//!    Address-san sad (of course it did). I could possibly try to do evil
//!    things like catch `SIGSEGV` and then try again with `libc::free`. Or
//!    somehow hook the library and override `libc`'s allocation facilities. But
//!    I am (at least for now) not willing to risk such fragile solution.

#![forbid(unsafe_op_in_unsafe_fn)]
#![deny(missing_docs)]

pub mod progress;
pub mod string;
pub mod wim;

mod error;
mod raii;

#[doc(inline)]
pub use wim::{
	AddFlags, CompressionType, DeleteFlags, DirEntry, ExportFlags, ExtractFlags, FileAttributes,
	Image, ImageIndex, IterateDirTreeFlags, IterateLookupTableFlags, LazyStreamEntry, ObjectId,
	OpenFlags, ReferenceFlags, ReferenceTemplateImageFlags, RenameFlags, ReparseTag, ResourceEntry,
	ResourceWimRef, SetInfo, SourceTargetPair, StreamEntry, UpdateCommand, UpdateFlags, Wim,
	WimInfo, WriteFlags, ALL_IMAGES,
};

pub use {error::Error, raii::*, wimlib_sys as sys};

/// Return the version of wimlib
pub fn version() -> (u16, u16, u16) {
	let version_int = unsafe { sys::wimlib_get_version() };
	let patch = version_int & ((1 << 10) - 1);
	let minor = (version_int >> 10) & ((1 << 10) - 1);
	let major = (version_int >> 20) & ((1 << 12) - 1);

	(major as u16, minor as u16, patch as u16)
}

// NOTE:
// Not sure if even include error file stuff in wrapped bindings
// and not just use progress messages and use established tracing facilities
