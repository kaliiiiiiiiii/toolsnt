use {
	crate::{elements::DynamicElement, value::GetValue},
	derive_more::{Display, Error},
	error_stack::{Result, ResultExt},
	hivex::{alloc::LibCAlloc, node::NodeHandle, BorrowedHive, LibCBox},
};

macro_rules! try_some {
	($expr:expr) => {
		match $expr {
			Ok(o) => o,
			Err(e) => return Some(Err(e.into())),
		}
	};
}

pub type Pair<'a> = (DynamicElement, GetValue<'a>);
type HandlesIter = allocator_api2::vec::IntoIter<NodeHandle, LibCAlloc>;

pub struct Elements<'hive> {
	pub(super) hive: BorrowedHive<'hive>,
	pub(super) handles: HandlesIter,
}

impl<'hive> Elements<'hive> {
	pub fn with_values(self) -> ElementValuePairs<'hive> {
		ElementValuePairs {
			hive: self.hive,
			handles: self.handles,
		}
	}

	pub fn len(&self) -> usize {
		self.handles.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl Iterator for Elements<'_> {
	type Item = Result<DynamicElement, IdParseError>;

	fn next(&mut self) -> Option<Self::Item> {
		let handle = self.handles.next()?;
		let name = try_some!(self.hive.node(handle).name().change_context(IdParseError));
		let id = try_some!(u32::from_str_radix(&name, 16).change_context(IdParseError));

		Some(Ok(DynamicElement::new(id)))
	}
}

pub struct ElementValuePairs<'hive> {
	hive: BorrowedHive<'hive>,
	handles: HandlesIter,
}

impl ElementValuePairs<'_> {
	pub fn len(&self) -> usize {
		self.handles.len()
	}

	pub fn is_empty(&self) -> bool {
		self.len() == 0
	}
}

impl<'hive> Iterator for ElementValuePairs<'hive> {
	type Item = Result<Pair<'hive>, PairsError>;

	fn next(&mut self) -> Option<Self::Item> {
		let handle = self.handles.next()?;
		let name = self.hive.node(handle).name().ok()?;
		let id =
			try_some!(u32::from_str_radix(&name, 16)
				.change_context_lazy(|| PairsError::NameParse { name }));

		let element = DynamicElement::new(id);

		let value = try_some!(super::node_get_value(element, self.hive.clone(), handle)
			.change_context_lazy(|| PairsError::ValueDecode { element }))?;

		Some(Ok((element, value)))
	}
}

#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub struct IdParseError;

#[derive(Debug, Display, Error, PartialEq, Eq)]
pub enum PairsError {
	#[display("Failed to decode name \"{name}\"")]
	NameParse { name: LibCBox<str> },
	#[display("Failed to decode value")]
	ValueDecode { element: DynamicElement },
}
