use {
	crate::elements::DynamicElement,
	derive_more::{Display, Error},
	error_stack::{Result, ResultExt},
	hivex::{alloc::LibCAlloc, node::NodeHandle, BorrowedHive, Hive},
};

macro_rules! try_some {
	($expr:expr) => {
		match $expr {
			Ok(o) => o,
			Err(e) => return Some(Err(e.into())),
		}
	};
}

type HandlesIter = allocator_api2::vec::IntoIter<NodeHandle, LibCAlloc>;

pub struct Elements<'hive> {
	pub(super) hive: BorrowedHive<'hive>,
	pub(super) handles: HandlesIter,
}

impl Iterator for Elements<'_> {
	type Item = Result<DynamicElement, KeyParseError>;

	fn next(&mut self) -> Option<Self::Item> {
		let handle = self.handles.next()?;
		let name = try_some!(self.hive.node(handle).name().change_context(KeyParseError));

		let id = try_some!(u32::from_str_radix(&name, 16).change_context(KeyParseError));
		Some(Ok(DynamicElement::new(id)))
	}
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub struct KeyParseError;
