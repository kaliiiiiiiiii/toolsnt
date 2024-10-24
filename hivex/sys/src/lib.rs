//! Raw FFI bindings for Hivex, generated using `bindgen`

#![allow(non_camel_case_types, non_upper_case_globals)]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

pub const VERSION: &str = env!("LIB_VERSION");
