pub use super::device::DeviceFormat;

use {
	derive_more::{Display, Error},
	error_stack::{ensure, report, Result, ResultExt},
	hivex::{value::Value as HiveValue, LibCBox},
	std::{
		borrow::Cow,
		ffi::{CStr, CString},
	},
	uuid::Uuid,
};

pub trait Format {
	type Get;
	type Set<'a>;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError>;
	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>>;
}

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

pub enum Integer {}
impl Format for Integer {
	type Get = u64;
	type Set<'a> = u64;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;

		let array_result: std::result::Result<[u8; 8], _> = bytes[..].try_into();
		let array = array_result
			.change_context(FromHiveValueError::Format)
			.attach_printable("Integer values should have 8 bytes.")?;

		Ok(u64::from_le_bytes(array))
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		let bytes = value.to_le_bytes();
		HiveValue::Binary(LibCBox::from(&bytes[..]))
	}
}

pub enum Bool {}
impl Format for Bool {
	type Get = bool;
	type Set<'a> = bool;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;
		match &bytes[..] {
			[0] => Ok(false),
			[1] => Ok(true),
			[_] => Err(report!(FromHiveValueError::Format)
				.attach_printable("Boolean can be represented only by 0 or 1")),
			[..] => Err(report!(FromHiveValueError::Format).attach_printable(
				"Boolean value representation as REG_BINARY should have only one byte",
			)),
		}
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		let slice = &[value as u8][..];
		HiveValue::Binary(LibCBox::from(slice))
	}
}

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

pub enum IntegerList {}
impl Format for IntegerList {
	type Get = LibCBox<[u64]>;
	type Set<'a> = &'a [u64];

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;

		// We are doing raw casting. So we want it to actually have entire u64s inside.
		ensure!(
			bytes.len() % size_of::<u64>() == 0,
			report!(FromHiveValueError::Format)
				.attach_printable("REG_BINARY representation of integer list's length is not a multiply of eight (64 bits)")
		);

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

#[derive(Clone, Copy, Debug, Display, Error)]
pub enum FromHiveValueError {
	#[display("The type of hive value doesn't match the format's expected one")]
	TypeMismatch,
	#[display("Value's format doesn't correspond the expected one")]
	Format,
	#[display("BCDEdit doesn't support this feature")]
	NotImplemented,
}

fn sz_to_uuid(string: LibCBox<str>) -> Result<Uuid, FromHiveValueError> {
	ensure!(
		string.starts_with('{') && string.ends_with('}'),
		report!(FromHiveValueError::Format)
			.attach_printable("Sanity check — UUIDs should be braced")
	); // note: meanwhile the author of this crate doesn't pass any sanity check

	Uuid::parse_str(&string).change_context(FromHiveValueError::Format)
}

fn uuid_to_sz(uuid: Uuid) -> Cow<'static, CStr> {
	let braced = uuid.braced().to_string();
	let cstring = CString::new(braced).expect("Formated UUID should not contain NUL");
	cstring.into()
}

macro_rules! extract {
	($expr:expr, $variant:ident) => {
		match $expr {
			HiveValue::$variant(inner) => Ok(inner),
			_ => Err(::error_stack::report!(FromHiveValueError::TypeMismatch)
				.attach_printable(concat!("Expected `", stringify!($variant), "`"))),
		}
	};
}

pub(super) use extract;
