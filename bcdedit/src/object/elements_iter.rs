//! Iterables through elements of an object

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

type Pair<'a> = (DynamicElement, GetValue<'a>);
type HandlesIter = allocator_api2::vec::IntoIter<NodeHandle, LibCAlloc>;

/// Iterator through elements
pub struct Elements<'hive> {
	pub(super) hive: BorrowedHive<'hive>,
	pub(super) handles: HandlesIter,
}

impl<'hive> Elements<'hive> {
	/// Attach values and form pairs
	pub fn with_values(self) -> ElementValuePairs<'hive> {
		ElementValuePairs {
			hive: self.hive,
			handles: self.handles,
		}
	}

	/// Returns the exact remaining length of the iterator
	///
	/// Refer to [`ExactSizeIterator::len`]
	pub fn len(&self) -> usize {
		self.handles.len()
	}

	/// Check if there are not elements
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

/// Iterate through pairs of elements and their values
pub struct ElementValuePairs<'hive> {
	hive: BorrowedHive<'hive>,
	handles: HandlesIter,
}

impl ElementValuePairs<'_> {
	/// Returns the exact remaining length of the iterator
	///
	/// Refer to [`ExactSizeIterator::len`]
	pub fn len(&self) -> usize {
		self.handles.len()
	}

	/// Check if there are not elements
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

/// Failed to parse the ID
#[derive(Clone, Copy, Debug, Display, Error, PartialEq, Eq)]
pub struct IdParseError;

/// Error when retreiving pairs
#[derive(Debug, Display, Error, PartialEq, Eq)]
pub enum PairsError {
	/// Failed to parse element's name
	#[display("Failed to decode name \"{name}\"")]
	NameParse {
		/// The name in the hive
		name: LibCBox<str>,
	},
	/// Failed to decode the value
	#[display("Failed to decode value")]
	ValueDecode {
		/// Element which BCDEdit failed to decode
		element: DynamicElement,
	},
}
