//! BCD value formats

pub use super::device::DeviceFormat;
use {
	derive_more::{Display, Error},
	hivex::{value::Value as HiveValue, LibCBox},
	std::{
		borrow::Cow,
		ffi::{CStr, CString},
	},
	uuid::Uuid,
};

/// Defines BCD format
pub trait Format {
	/// Type when getting
	type Get;
	/// Type when setting
	type Set<'a>;

	/// Convert value from [`hivex`] to [`Self::Get`]
	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError>;
	/// Convert [`Self::Set`] to [`hivex`] value to be saved
	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>>;
}

/// A string (with internal UTF-16 representation)
pub enum Str {}
impl Format for Str {
	type Get = LibCBox<str>;
	type Set<'a> = Cow<'a, CStr>;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		extract!(value, Sz)
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		HiveValue::Sz(value)
	}
}

/// An 64-bit unsigned integer
pub enum Integer {}
impl Format for Integer {
	type Get = u64;
	type Set<'a> = u64;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;
		let array = bytes[..]
			.try_into()
			.map_err(|_| PatternMismatchInner::IntSize)
			.map_err(FromHiveValueError::pattern)?;

		Ok(u64::from_le_bytes(array))
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		let bytes = value.to_le_bytes();
		HiveValue::Binary(LibCBox::from(&bytes[..]))
	}
}

/// Boolean value
pub enum Bool {}
impl Format for Bool {
	type Get = bool;
	type Set<'a> = bool;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;
		match &bytes[..] {
			[0] => Ok(false),
			[1] => Ok(true),
			[_] => Err(FromHiveValueError::pattern(
				PatternMismatchInner::BooleanMarkerValue,
			)),
			[..] => Err(FromHiveValueError::pattern(
				PatternMismatchInner::BooleanMarkerSize,
			)),
		}
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		let slice = &[value as u8][..];
		HiveValue::Binary(LibCBox::from(slice))
	}
}

/// Fancy Microsoft word for UUID
pub enum Guid {}
impl Format for Guid {
	type Get = Uuid;
	type Set<'a> = Uuid;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		sz_to_uuid(extract!(value, Sz)?)
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		HiveValue::Sz(uuid_to_sz(value))
	}
}

/// Fancy Microsoft word for UuidList
pub enum GuidList {}
impl Format for GuidList {
	type Get = Box<[Uuid]>;
	type Set<'a> = &'a [Uuid];

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let strings = extract!(value, MultiSz)?;
		strings.into_vec().into_iter().map(sz_to_uuid).collect()
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		HiveValue::MultiSz(value.iter().copied().map(uuid_to_sz).collect())
	}
}

/// List of [`integers`][`Integer`]
pub enum IntegerList {}
impl Format for IntegerList {
	type Get = LibCBox<[u64]>;
	type Set<'a> = &'a [u64];

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;

		let is_valid_layout = bytes.len() % size_of::<u64>() == 0;
		if !is_valid_layout {
			return Err(FromHiveValueError::pattern(
				PatternMismatchInner::IntListSize,
			));
		}

		let vector = bytes.into_vec();
		assert_eq!(
			vector.len(),
			vector.capacity(),
			"Assumed in Box<[T]> → Vec<T>"
		);
		let (ptr, len, _) = vector.into_raw_parts();
		let new_len = len / size_of::<u64>();
		let int_list = unsafe {
			allocator_api2::vec::Vec::from_raw_parts_in(
				ptr.cast::<u64>(),
				new_len,
				new_len,
				hivex::alloc::LibCAlloc,
			)
			.into_boxed_slice()
		};

		Ok(int_list)
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		let bytes = unsafe {
			std::slice::from_raw_parts(value.as_ptr().cast::<u8>(), std::mem::size_of_val(value))
		};

		HiveValue::Binary(bytes.into())
	}
}

fn sz_to_uuid(string: LibCBox<str>) -> Result<Uuid, FromHiveValueError> {
	let is_braced = string.starts_with('{') && string.ends_with('}');
	if !is_braced {
		return Err(FromHiveValueError::pattern(
			PatternMismatchInner::UuidNotBraced,
		));
	}

	Uuid::parse_str(&string)
		.map_err(|e| FromHiveValueError::pattern(PatternMismatchInner::UuidParse(e)))
}

fn uuid_to_sz(uuid: Uuid) -> Cow<'static, CStr> {
	let braced = uuid.braced().to_string();
	let cstring = CString::new(braced).expect("Formated UUID should not contain NUL");
	cstring.into()
}

/// Error when value can't be converted from [`hivex`] value
#[derive(Debug, Display, Error)]
pub enum FromHiveValueError {
	/// Value has different type than required
	#[display(r#"Type mismatch: Expected "{expected:?}", found "{found:?}""#)]
	TypeMismatch {
		/// Required type
		expected: ValueType,
		/// Type of the item
		found: ValueType,
	},
	/// Value's internal format doesn't match expected pattern
	#[display("Invalid pattern: {_0}")]
	Pattern(#[error(source)] PatternMismatch),
}

impl FromHiveValueError {
	/// Data pattern mismatch constructor
	pub(super) fn pattern(inner: PatternMismatchInner) -> Self {
		Self::Pattern(PatternMismatch(inner))
	}
}

/// Error on mismatch of data patterns
#[derive(Debug, Error)]
pub struct PatternMismatch(#[error(not(source))] pub(super) PatternMismatchInner);
impl Display for PatternMismatch {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		Display::fmt(&self.0, f)
	}
}

#[derive(Debug, Display)]
pub(super) enum PatternMismatchInner {
	#[display("Boolean value marker is neither 0 or 1")]
	BooleanMarkerValue,
	#[display("Boolean value representation as REG_BINARY should have only one byte")]
	BooleanMarkerSize,
	#[display(
		"REG_BINARY representation of integer list's length is not a multiply of eight (64 bits)"
	)]
	IntListSize,
	#[display("Integer values should have 8 bytes")]
	IntSize,
	#[display("Sanity check — UUIDs should be braced")]
	// note: meanwhile the author of this crate doesn't pass any sanity check
	UuidNotBraced,
	#[display("Invalid UUID: {_0}")]
	UuidParse(uuid::Error),
	#[display("Invalid device pattern: {_0}")]
	Device(binrw::Error),
}

macro_rules! extract {
	($expr:expr, $variant:ident) => {{
		let expr = $expr;
		match expr {
			HiveValue::$variant(inner) => Ok(inner),
			_ => Err(FromHiveValueError::TypeMismatch {
				expected: ::hivex::value::ValueType::$variant,
				found: expr.type_of(),
			}),
		}
	}};
}

pub(super) use extract;
use hivex::value::ValueType;
