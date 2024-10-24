pub mod device;

pub use device::DeviceFormat as Device;

use {
	derive_more::{Display, Error},
	error_stack::{bail, report, Report, Result, ResultExt},
	hivex::{
		value::{Value, ValueString},
		LibCBox,
	},
	std::{
		borrow::Cow,
		ffi::{CStr, CString},
	},
	uuid::Uuid,
};

#[derive(Clone, Copy, Debug, Display, Error)]
pub enum FromHiveValueError {
	#[display = "The type of hive value doesn't match the format's expected one"]
	TypeMismatch,
	#[display = "Value's format doesn't correspond the expected one"]
	Format,
	#[display = "BCDEdit doesn't support this feature"]
	NotImplemented,
}

/// A String format
pub enum String {}
pub trait Format {
	type Get;
	type Set<'a>;

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError>;
	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>>;
}

impl Format for String {
	type Get = LibCBox<str>;
	type Set<'a> = Cow<'a, CStr>;

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		match value {
			Value::Sz(string) => Ok(string),
			_ => bail!(FromHiveValueError::TypeMismatch),
		}
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		Value::Sz(value)
	}
}

impl Format for u64 {
	type Get = u64;
	type Set<'a> = u64;

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let Value::Binary(binary) = value else {
			bail!(FromHiveValueError::TypeMismatch);
		};

		let byte_array_result: std::result::Result<[u8; 8], _> = binary[..].try_into();
		let byte_array = byte_array_result.change_context(FromHiveValueError::Format)?;

		Ok(u64::from_le_bytes(byte_array))
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		let byte_array = value.to_le_bytes();
		Value::Binary(LibCBox::from(&byte_array[..]))
	}
}

impl Format for bool {
	type Get = bool;
	type Set<'a> = bool;

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let Value::Binary(binary) = value else {
			bail!(FromHiveValueError::TypeMismatch);
		};

		if binary.len() != 1 {
			let report = report!(FromHiveValueError::Format).attach_printable(
				"Boolean value represenation as REG_BINARY should have only one byte",
			);

			return Err(report);
		}

		match binary[0] {
			0 => Ok(false),
			1 => Ok(true),
			_ => {
				let report = report!(FromHiveValueError::Format)
					.attach_printable("Boolean can be represented only by 0 or 1 byte value");

				Err(report)
			}
		}
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		let slice = &[value as u8][..];
		Value::Binary(LibCBox::from(slice))
	}
}

pub enum Guid {}
impl Format for Guid {
	type Get = Uuid;
	type Set<'a> = Uuid;

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let Value::Sz(string) = value else {
			bail!(FromHiveValueError::TypeMismatch);
		};

		Uuid::parse_str(&string).change_context(FromHiveValueError::Format)
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		let braced = value.braced().to_string();
		let cstring = CString::new(braced).expect("Formatted UUID should not contain NUL");
		Value::Sz(cstring.into())
	}
}

pub enum GuidList {}
impl Format for GuidList {
	type Get = Box<[Uuid]>;
	type Set<'a> = &'a [Uuid];

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let Value::MultiSz(strings) = value else {
			bail!(FromHiveValueError::TypeMismatch);
		};

		let uuid_vec: Vec<_> = strings
			.iter()
			.map(|string| Uuid::parse_str(string).map_err(Report::from))
			.collect::<Result<_, _>>()
			.change_context(FromHiveValueError::Format)?;

		Ok(uuid_vec.into_boxed_slice())
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		let str_vec = value
			.iter()
			.map(|uuid| {
				let braced = uuid.braced().to_string();
				let c_string = CString::new(braced).expect("Formatted UUID should not contain NUL");
				Cow::Owned(c_string)
			})
			.collect();

		Value::MultiSz(str_vec)
	}
}

pub enum IntegerList {}
impl Format for IntegerList {
	type Get = LibCBox<[u64]>;
	type Set<'a> = &'a [u64];

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let Value::Binary(bytes) = value else {
			bail!(FromHiveValueError::TypeMismatch);
		};

		let ints = boxed_slice_try_call_with_as_mut_slice(bytes, |bytes| {
			bytemuck::try_cast_slice_mut::<_, u64>(bytes).map_err(Report::from)
		})
		.change_context(FromHiveValueError::Format)?;

		Ok(ints)
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		let bytes = bytemuck::cast_slice(value);
		let boxed = LibCBox::from(bytes);

		Value::Binary(boxed)
	}
}

pub trait IntoFormatSet<'a, T: Format + 'a> {
	fn into_format_set(self) -> T::Set<'a>;
}

macro_rules! id_impl_into_format_set {
	($($ty:ty),* $(,)?) => {
		$(
			impl<'a> IntoFormatSet<'a, $ty> for $ty {
				fn into_format_set(self) -> <$ty as Format>::Set<'a> {
					self
				}
			}
		)*
	};
}

id_impl_into_format_set!(u64, bool);

impl<'a> IntoFormatSet<'a, GuidList> for &'a [Uuid] {
	fn into_format_set(self) -> <GuidList as Format>::Set<'a> {
		self
	}
}

impl<'a> IntoFormatSet<'a, IntegerList> for &'a [u64] {
	fn into_format_set(self) -> <IntegerList as Format>::Set<'a> {
		self
	}
}

impl<'a, Str> IntoFormatSet<'a, String> for Str
where
	Str: ValueString + 'a,
{
	fn into_format_set(self) -> <String as Format>::Set<'a> {
		self.into_c_string()
	}
}

macro_rules! enum_formats {
	(
		$(
			$vis:vis enum $name:ident {
				$(
					$variant:ident = $value:expr
				),*
				$(,)?
			}
		)*
	) => {
		$(
			#[derive(Clone, Copy, Debug, PartialEq, Eq)]
			#[repr(u8)]
			$vis enum $name {
				$($variant = $value,)*
			}

			impl $crate::element::format::Format for $name {
				type Get     = $name;
				type Set<'a> = $name;

				fn from_hive_value(value: ::hivex::value::Value<::hivex::LibCBox<str>>)
					-> ::error_stack::Result<Self::Get, $crate::element::format::FromHiveValueError>
				{
					let num = u64::from_hive_value(value)?;

					match num {
						$($value => Ok(Self::$variant),)*
						_ => {
							let report = ::error_stack::report!($crate::element::format::FromHiveValueError::Format)
								.attach_printable("Enum value out of bounds");

							Err(report)
						},
					}
				}

				fn into_hive_value(value: Self::Set<'_>) -> ::hivex::value::Value<::std::borrow::Cow<'_, ::std::ffi::CStr>> {
					let num = value as u64;
					u64::into_hive_value(num)
				}
			}

			impl<'a> $crate::element::format::IntoFormatSet<'a, $name> for $name {
				fn into_format_set(self) -> <$name as $crate::element::Format>::Set<'a> {
					self
				}
			}
		)*
	};
}

pub(super) use enum_formats;

fn boxed_slice_try_call_with_as_mut_slice<T, U, E>(
	slice: LibCBox<[T]>,
	f: impl Fn(&mut [T]) -> Result<&mut [U], E>,
) -> Result<LibCBox<[U]>, E> {
	use allocator_api2::vec::Vec;

	let (ptr, len, _) = slice.into_vec().into_raw_parts();
	let slice = unsafe { std::slice::from_raw_parts_mut(ptr, len) };

	let new_slice = f(slice)?;
	let len = new_slice.len();

	let new_vec = unsafe {
		Vec::from_raw_parts_in(new_slice.as_mut_ptr(), len, len, hivex::alloc::LibCAlloc)
	};

	Ok(new_vec.into_boxed_slice())
}
