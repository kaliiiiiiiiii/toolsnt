//! Object »properties«

pub mod application;
pub mod device;
pub mod format;
pub mod library;

use format::Format;

/// BCD Object Element
pub trait Element {
	/// Element key in the BCD registry hive
	const ID: &'static str;

	/// Element value datatype
	type Format: Format;

	/// Type of object the element is of
	type Class;
}

macro_rules! define_elements {
	{
		for $class:ty;
		$($name:ident: $format:ty = $const:expr;)*
	} => {
		$(
			pub struct $name;
			impl $crate::element::Element for $name {
				const ID: &'static str = concat!($const, "\0");
				type Format = $format;
				type Class  = $class;
			}
		)*
	};
}

use define_elements;
