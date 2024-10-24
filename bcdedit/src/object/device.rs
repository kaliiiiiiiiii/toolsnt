use crate::typesystem::{is_subclass_of_self, SubclassOf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Device;
is_subclass_of_self!(Device);

impl SubclassOf<super::Any> for Device {
	fn upcast(self) -> super::Any {
		super::Any::Device(self)
	}

	fn downcast_from(above: super::Any) -> Result<Self, super::Any>
	where
		Self: Sized,
	{
		if let super::Any::Device(device) = above {
			Ok(device)
		} else {
			Err(above)
		}
	}
}
