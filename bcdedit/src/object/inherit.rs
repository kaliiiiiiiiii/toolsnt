use crate::typesystem::{DowncastExt, SubclassOf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Inherit<For> {
	pub(crate) inherit_for: For,
}

pub mod inherit_for {
	crate::typesystem::def_markers!(All, Application, Device);
}

impl<For> SubclassOf<super::Any> for Inherit<For>
where
	For: SubclassOf<inherit_for::Any>,
{
	fn upcast(self) -> super::Any {
		super::Any::Inherit(self.upcast())
	}

	fn downcast_from(above: super::Any) -> Result<Self, super::Any>
	where
		Self: Sized,
	{
		let super::Any::Inherit(inherit) = above else {
			return Err(above);
		};

		match inherit.downcast() {
			Ok(ok) => Ok(ok),
			Err(_) => Err(above),
		}
	}
}

impl<SuperF, SubF> SubclassOf<Inherit<SuperF>> for Inherit<SubF>
where
	SubF: SubclassOf<SuperF>,
{
	fn upcast(self) -> Inherit<SuperF> {
		Inherit {
			inherit_for: self.inherit_for.upcast(),
		}
	}

	fn downcast_from(above: Inherit<SuperF>) -> Result<Self, Inherit<SuperF>>
	where
		Self: Sized,
	{
		match above.inherit_for.downcast() {
			Ok(inherit_for) => Ok(Self { inherit_for }),
			Err(inherit_for) => Err(Inherit { inherit_for }),
		}
	}
}

pub trait CanInherit<I> {}

impl<AppType> CanInherit<inherit_for::Application> for super::application::Application<AppType> {}
impl CanInherit<inherit_for::Device> for super::device::Device {}
impl<T> CanInherit<inherit_for::All> for T {}
impl<T> CanInherit<inherit_for::Any> for T {}
