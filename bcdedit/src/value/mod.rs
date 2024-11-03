//! Rust representations of BCD's values
//!
//! # Rant
//! BCD is both overengineered and underengineered. Windows Registry has funny
//! things, in my homeland we call them types.
//!
//! It does even have `REG_DWORD` type, an [integer][`format::Integer`] type.
//! But no, Microsoft has to pretend it doesn't exist and encode a DWORD
//! ([`u64`]) as `REG_BINARY` (slice of bytes).
//!
//! And you wouldn't guess how they encode [UUIDs][`format::Guid`]. You have
//! here your beloved `REG_BINARY`, Microsoft. It could have been 16 bytes. But
//! no! You had to choose UTF-16 pretty representation of UUID. The braced
//! format with the dashes. 39 bytes instead of 16.
//!
//! And don't get me started on the [Device][`device`] format.
//!
//! Congratulations, Microsoft. Good job. /s

pub mod device;
pub mod format;

mod value;

pub use value::{GetValue, SetValue, Type, Value};
