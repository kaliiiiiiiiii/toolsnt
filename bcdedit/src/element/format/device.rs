use {
	super::FromHiveValueError,
	binrw::{binrw, BinRead, BinWrite, NullWideString},
	error_stack::{bail, Result, ResultExt},
	hivex::{value::Value, LibCBox},
	std::{borrow::Cow, ffi::CStr, io::Cursor},
	uuid::Uuid,
};

type LibCVec<T> = allocator_api2::vec::Vec<T, hivex::alloc::LibcAlloc>;

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq, Eq)]
pub struct DeviceFormat {
	#[br(map = |arr: [u8; 16]| Uuid::from_bytes_le(arr))]
	#[bw(map = |uuid| uuid.to_bytes_le())]
	pub additional_options: Uuid,
	#[brw(pad_before = 8)]
	pub device: Device,
}

impl super::Format for DeviceFormat {
	type Get = Self;
	type Set<'a> = Self;

	fn from_hive_value(value: Value<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let Value::Binary(bytes) = value else {
			bail!(FromHiveValueError::TypeMismatch)
		};

		let mut reader = Cursor::new(bytes);
		Self::read(&mut reader).change_context(FromHiveValueError::Format)
	}

	fn into_hive_value(value: Self::Set<'_>) -> Value<Cow<'_, CStr>> {
		let mut writer = Cursor::new(vec![]);
		value.write(&mut writer).expect("Failed to write value");

		// FIXME: don't copy
		let mut new_bytes = LibCVec::new_in(hivex::alloc::LibcAlloc);
		new_bytes.extend_from_slice(&writer.into_inner());
		Value::Binary(new_bytes.into_boxed_slice())
	}
}

#[binrw]
#[brw(little)]
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Device {
	#[brw(magic = 0x48_u64)]
	Parition(Partition),
	#[brw(magic = 0x80_u64)]
	File(#[brw(magic = 0x05_u32)] File),
	#[brw(magic = 0x94_u64)]
	Ramdisk(Ramdisk),
	// #[brw(magic = 0xC6_u64)]
	// LocateEx,
}

#[binrw]
#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Partition {
	#[non_exhaustive]
	Mbr {
		#[brw(pad_before = 2)]
		#[br(temp)]
		#[bw(ignore)]
		__: (),

		#[brw(magic = 0x10_u8)]
		#[br(temp)]
		#[bw(ignore)]
		__: (),

		/// Partition index
		#[brw(pad_before = 17)]
		partition: u32,

		/// Disk ID
		#[brw(pad_after = 27)]
		disk: u32,
	},

	#[non_exhaustive]
	Gpt {
		/// Partition UUID
		#[br(map = |arr: [u8; 16]| Uuid::from_bytes_le(arr))]
		#[bw(map = |uuid| uuid.to_bytes_le())]
		partition: Uuid,

		/// Disk UUID
		#[brw(magic = 0_u64)]
		#[brw(pad_after = 16)]
		#[br(map = |arr: [u8; 16]| Uuid::from_bytes_le(arr))]
		#[bw(map = |uuid| uuid.to_bytes_le())]
		disk: Uuid,
	},
}

#[binrw]
#[non_exhaustive]
#[derive(Debug, PartialEq, Eq)]
pub struct File {
	// This is probably NOT magic
	// There should be one byte, but that one
	// varied in another test cases, specialised
	// in composites using File
	#[brw(magic = 0x01_u32)]
	#[br(temp)]
	#[bw(ignore)]
	__: (),

	#[brw(magic = 0x6C_u32)]
	#[br(temp)]
	#[bw(ignore)]
	__: (),

	#[brw(magic = 0x05_u32)]
	#[br(temp)]
	#[bw(ignore)]
	__: (),

	#[brw(magic = 0x06_u32)]
	#[br(temp)]
	#[bw(ignore)]
	__: (),

	#[brw(pad_before = 4)]
	pub device: Box<Device>,
	pub path: NullWideString,
}

#[binrw]
#[derive(Debug, PartialEq, Eq)]
pub struct Ramdisk {
	// Maybe mystery data
	#[brw(magic = b"\x03")]
	#[br(temp)]
	#[bw(ignore)]
	__: (),

	// Mystery data
	#[br(temp)]
	#[bw(calc = [0; 19])]
	__: [u8; 19],

	#[brw(magic = 0x00_u32)]
	pub file: File,
}

#[cfg(test)]
mod tests {
	use {super::*, uuid::uuid};

	#[test]
	fn partition_gpt() {
		let path = "Reverse Engineering/Test Data/partition.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		assert_eq!(
			format,
			DeviceFormat {
				additional_options: Uuid::nil(),
				device: Device::Parition(Partition::Gpt {
					partition: uuid!("6855e541-a1f1-479d-94bc-09ac08630a8b"),
					disk: uuid!("39ab146a-7277-46fa-be32-a80dda14de23")
				})
			}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		assert_eq!(writer.into_inner(), std::fs::read(path).unwrap());
	}

	#[test]
	fn partition_mbr() {
		let path = "Reverse Engineering/Test Data/partition-mbr.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		assert_eq!(
			format,
			DeviceFormat {
				additional_options: Uuid::nil(),
				device: Device::Parition(Partition::Mbr {
					disk: 0x0e8a4be6,
					partition: 1
				})
			}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		assert_eq!(writer.into_inner(), std::fs::read(path).unwrap());
	}

	#[test]
	fn file() {
		let path = "Reverse Engineering/Test Data/filedevice.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		assert_eq!(
			format,
			DeviceFormat {
				additional_options: Uuid::nil(),
				device: Device::File(File {
					device: Box::new(Device::Parition(Partition::Gpt {
						partition: uuid!("6855e541-a1f1-479d-94bc-09ac08630a8b"),
						disk: uuid!("39ab146a-7277-46fa-be32-a80dda14de23")
					})),
					path: NullWideString::from("\\Amogus.Bin"),
				})
			}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		assert_eq!(writer.into_inner(), std::fs::read(path).unwrap());
	}

	#[test]
	fn ramdisk() {
		let path = "Reverse Engineering/Test Data/ramdisk.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		assert_eq!(
			format,
			DeviceFormat {
				additional_options: uuid!("8e71a3c1-a8a2-493f-bebc-fdc2110ca739"),
				device: Device::Ramdisk(Ramdisk {
					file: File {
						device: Box::new(Device::Parition(Partition::Gpt {
							partition: uuid!("6855e541-a1f1-479d-94bc-09ac08630a8b"),
							disk: uuid!("39ab146a-7277-46fa-be32-a80dda14de23")
						})),
						path: NullWideString::from("\\Amogus.Bin"),
					}
				})
			}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		assert_eq!(writer.into_inner(), std::fs::read(path).unwrap());
	}
}
