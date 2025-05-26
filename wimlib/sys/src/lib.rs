//! Raw FFI bindings for wimlib, generated using `bindgen`

#![allow(
	non_snake_case,
	non_camel_case_types,
	non_upper_case_globals,
	unnecessary_transmutes
)]

include!(concat!(env!("OUT_DIR"), "/bindings.rs"));
