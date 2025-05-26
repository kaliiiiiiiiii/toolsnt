//! Raw FFI bindings for wimlib, generated using `bindgen`

#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	unnecessary_transmutes,
	clippy::missing_safety_doc,
	clippy::ptr_offset_with_cast,
	clippy::too_many_arguments,
	clippy::useless_transmute
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
