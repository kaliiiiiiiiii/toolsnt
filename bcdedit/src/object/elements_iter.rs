//! Iterables through elements of an object

use {
	super::RetrievalError,
	crate::{elements::DynamicElement, value::GetValue},
	derive_more::derive::{Display, Error},
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
	type Item = std::io::Result<DynamicElement>;

	fn next(&mut self) -> Option<Self::Item> {
		let handle = self.handles.next()?;
		let name = try_some!(self.hive.node(handle).name());
		let id = try_some!(u32::from_str_radix(&name, 16)
			.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e)));

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
	type Item = Result<Pair<'hive>, PairDecodeError>;

	fn next(&mut self) -> Option<Self::Item> {
		let (handle, name) = loop {
			let handle = self.handles.next()?;
			if let Ok(name) = self.hive.node(handle).name() {
				break (handle, name);
			}
		};

		let id = match u32::from_str_radix(&name, 16) {
			Ok(id) => id,
			Err(e) => return Some(Err(PairDecodeError::Id(e))),
		};

		let element = DynamicElement::new(id);
		let result = super::node_get_value(element, self.hive.clone(), handle)
			.transpose()?
			.map(|value| (element, value))
			.map_err(|error| PairDecodeError::Value { name, error });

		Some(result)
	}
}

/// Error when decoding value pair
#[derive(Debug, Display, Error)]
pub enum PairDecodeError {
	/// Failed to decode ID
	#[display("Failed to decode ID: {_0}")]
	Id(#[error(source)] std::num::ParseIntError),
	/// ID found but value is not valid
	#[display("Failed to decode value for \"{name}\": {error}")]
	Value {
		/// Name of the element
		name: LibCBox<str>,
		/// Backing error
		#[error(source)]
		error: RetrievalError,
	},
}
