//! The BCD type relationship transfered to Rust typesystem

/// Defines subclass relationship for a type
///
/// The type hiearchy is not transitive
pub trait SubclassOf<Above> {
	/// Cast to a parent type
	fn upcast(self) -> Above;

	/// Create from a parent type
	fn downcast_from(above: Above) -> Result<Self, Above>
	where
		Self: Sized;
}

/// Convenience extension for downcasting
pub trait DowncastExt {
	fn downcast<T>(self) -> Result<T, Self>
	where
		T: SubclassOf<Self>,
		Self: Sized;
}

impl<A> DowncastExt for A {
	fn downcast<T>(self) -> Result<T, Self>
	where
		T: SubclassOf<Self>,
		Self: Sized,
	{
		T::downcast_from(self)
	}
}

macro_rules! def_markers {
	($($name:ident),* $(,)?) => {
		$(
			#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
			pub struct $name;

			impl $crate::typesystem::SubclassOf<Any> for $name {
				fn upcast(self) -> Any {
					Any::$name
				}

				fn downcast_from(above: Any) -> Result<Self, Any> {
					if above == Any::$name {
						Ok(Self)
					} else {
						Err(above)
					}
				}
			}
		)*

		#[derive(Clone, Copy, Debug, PartialEq, Eq)]
		pub enum Any {
			$($name,)*
		}

		$crate::typesystem::is_subclass_of_self!(Any);
	};
}

macro_rules! is_subclass_of_self {
	($ty:ty) => {
		impl $crate::typesystem::SubclassOf<$ty> for $ty {
			fn upcast(self) -> Self {
				self
			}

			fn downcast_from(above: $ty) -> std::result::Result<Self, Self> {
				Ok(above)
			}
		}
	};
}

pub(crate) use {def_markers, is_subclass_of_self};
