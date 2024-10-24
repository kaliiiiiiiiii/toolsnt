use {
	hivex::{Hive, OpenFlags},
	std::{
		ops::{Deref, DerefMut},
		path::{Path, PathBuf},
	},
	uuid::Uuid,
};

pub struct TestHive {
	path: PathBuf,
	hive: Hive,
}

impl Deref for TestHive {
	type Target = Hive;

	fn deref(&self) -> &Self::Target {
		&self.hive
	}
}

impl DerefMut for TestHive {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.hive
	}
}

impl Drop for TestHive {
	fn drop(&mut self) {
		let _ = std::fs::remove_file(&self.path);
	}
}

impl TestHive {
	pub fn new(write: bool) -> std::io::Result<Self> {
		let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
			.join(format!("{}.dat", Uuid::new_v4().as_hyphenated()));

		let flags = write.then_some(OpenFlags::WRITE).unwrap_or_default();
		let hive = Hive::create(&path, flags)?;

		Ok(TestHive { path, hive })
	}
}
