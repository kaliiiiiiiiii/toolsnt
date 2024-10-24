use crate::typesystem::{DowncastExt, SubclassOf};

/// Application entry object
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Application<Type> {
	pub(crate) application_type: Type,
	pub(crate) image_type: ImageType,
}

impl<Type> Application<Type> {
	pub const fn new(application_type: Type, image_type: ImageType) -> Self {
		Self {
			application_type,
			image_type,
		}
	}

	/// Get current application type
	pub const fn application_type(&self) -> &Type {
		&self.application_type
	}

	/// Get image type
	pub const fn image_type(&self) -> ImageType {
		self.image_type
	}
}

impl<Type: Default> Application<Type> {
	pub fn with_default_application_type(image_type: ImageType) -> Self {
		Self {
			application_type: Default::default(),
			image_type,
		}
	}
}

/// Application image type
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ImageType {
	Firmware,
	WindowsBoot,
	LegacyLoader,
	RealMode,
}

/// Application types
pub mod app_type {
	crate::typesystem::def_markers!(
		FwBootMgr, BootMgr, OsLoader, Resume, MemDiag, NtLdr, SetupLdr, BootSector, Startup,
		BootApp,
	);
}

/// Defines subclassing based of subclassing of inner type
impl<Type> SubclassOf<super::Any> for Application<Type>
where
	Type: SubclassOf<app_type::Any>,
{
	fn upcast(self) -> super::Any {
		super::Any::Application(self.upcast())
	}

	fn downcast_from(above: super::Any) -> Result<Self, super::Any>
	where
		Self: Sized,
	{
		let super::Any::Application(app) = above else {
			return Err(above);
		};

		match app.downcast() {
			Ok(ok) => Ok(ok),
			Err(_) => Err(above),
		}
	}
}

/// Defines application of type SubT, which is subclass of SuperT
/// to be a subclass of application of SuperT
impl<SuperT, SubT> SubclassOf<Application<SuperT>> for Application<SubT>
where
	SubT: SubclassOf<SuperT>,
{
	fn upcast(self) -> Application<SuperT> {
		Application {
			application_type: self.application_type.upcast(),
			image_type: self.image_type,
		}
	}

	fn downcast_from(above: Application<SuperT>) -> Result<Self, Application<SuperT>>
	where
		Self: Sized,
	{
		let image_type = above.image_type;

		match above.application_type.downcast() {
			Ok(application_type) => Ok(Self {
				application_type,
				image_type,
			}),
			Err(application_type) => Err(Application {
				application_type,
				image_type,
			}),
		}
	}
}
