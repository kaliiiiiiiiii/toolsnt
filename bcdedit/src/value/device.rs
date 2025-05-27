//! Represents a location (like disk, virtual disk image, …)
//!
//! # Rant
//! After everything Microsoft has done we saw… this is probably the worst.
//! Imagine having a structured key-value database and just shove encoded
//! mysterious data structure in `REG_BINARY`.

use {
	super::format::{extract, Format, FromHiveValueError, PatternMismatchInner},
	binrw::{binrw, BinRead, BinWrite, NullWideString},
	hivex::{value::Value as HiveValue, LibCBox},
	std::{borrow::Cow, ffi::CStr, io::Cursor},
	uuid::Uuid,
};

type LibCVec<T> = allocator_api2::vec::Vec<T, hivex::alloc::LibCAlloc>;

/// [`Device`] with additional options
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DeviceFormat {
	/// Object UUID with additional device options
	#[br(map = |arr: [u8; 16]| Uuid::from_bytes_le(arr))]
	#[bw(map = |uuid| uuid.to_bytes_le())]
	pub additional_options: Uuid,
	/// Device itself
	pub device: Device,
}

/// BCD device
#[binrw]
#[brw(little)]
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Device {
	/// Plain partition on a disk
	#[brw(magic = 0x06_u64)]
	Partition(#[brw(magic = 0x48_u64)] Partition),
	/// File
	#[brw(magic = b"\0\0\0\0\0\0\0\0\x80\0\0\0\0\0\0\0")]
	File(#[brw(magic = 0x05_u32)] File),
	/// Ramdisk (unknown to me)
	#[brw(magic = b"\0\0\0\0\0\0\0\0\x94\0\0\0\0\0\0\0")]
	Ramdisk(Ramdisk),
	// #[brw(magic = 0xC6_u64)]
	// LocateEx,
}

/// Partition on disk
#[binrw]
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum Partition {
	/// Master Boot Record partition
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

	/// GUID Partition Table
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

impl Partition {
	/// Create [`Self::Mbr`] variant
	///
	/// This is required as all other fields are not known yet
	pub fn mbr(disk: u32, partition: u32) -> Self {
		Self::Mbr { partition, disk }
	}

	/// Create [`Self::gpt`] variant
	///
	/// This is required as all other fields are not known yet
	pub fn gpt(disk: Uuid, partition: Uuid) -> Self {
		Self::Gpt { partition, disk }
	}
}

/// A file on a device
#[binrw]
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, Eq)]
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

	/// Device on which the file is located
	pub device: Box<Device>,
	/// Path to the file (UTF-16)
	pub path: NullWideString,
}

impl File {
	/// Create a new file
	pub fn new(device: Box<Device>, path: NullWideString) -> Self {
		Self { device, path }
	}
}

/// Ramdisk file
#[binrw]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ramdisk {
	// Maybe mystery data
	#[brw(magic = b"\x03")]
	#[br(temp)]
	#[bw(ignore)]
	__: (),

	// Uncle Bill's Mystery Bytes
	#[br(temp)]
	#[bw(calc = [0; 19])]
	__: [u8; 19],

	/// File with ramdisk
	#[brw(magic = 0x00_u32)]
	pub file: File,
}

impl Format for DeviceFormat {
	type Get = Self;
	type Set<'a> = Self;

	fn from_hive_value(value: HiveValue<LibCBox<str>>) -> Result<Self::Get, FromHiveValueError> {
		let bytes = extract!(value, Binary)?;
		let mut reader = Cursor::new(bytes);
		Self::read(&mut reader)
			.map_err(PatternMismatchInner::Device)
			.map_err(FromHiveValueError::pattern)
	}

	fn into_hive_value(value: Self::Set<'_>) -> HiveValue<Cow<'_, CStr>> {
		let mut writer = Cursor::new(vec![]);
		value.write(&mut writer).expect("Failed to write value");

		// hack: copies
		let vector = writer.into_inner();
		let mut new_bytes = LibCVec::with_capacity_in(vector.len(), hivex::alloc::LibCAlloc);
		new_bytes.extend_from_slice(&vector);
		HiveValue::Binary(new_bytes.into_boxed_slice())
	}
}

#[cfg(test)]
mod tests {
	use {super::*, assert2::check, std::io::Seek, uuid::uuid};

	const GPT_DEV: Device = Device::Partition(Partition::Gpt {
		disk: uuid!("39ab146a-7277-46fa-be32-a80dda14de23"),
		partition: uuid!("6855e541-a1f1-479d-94bc-09ac08630a8b"),
	});

	const MBR_DEV: Device = Device::Partition(Partition::Mbr {
		partition: 1,
		disk: 0x0e8a4be6,
	});

	const IMAGE_PATH: &str = "\\Amogus.Bin";

	#[test]
	fn encode_decode() {
		let device = DeviceFormat {
			additional_options: Uuid::nil(),
			device: GPT_DEV,
		};

		let mut curosr = Cursor::new(vec![]);
		device.write(&mut curosr).unwrap();
		curosr.rewind().unwrap();
		let read = DeviceFormat::read(&mut curosr).unwrap();

		check!(read == device);
	}

	#[test]
	fn partition_gpt() {
		let path = "Reverse Engineering/Test Data/partition.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		check!(
			format
				== DeviceFormat {
					additional_options: Uuid::nil(),
					device: GPT_DEV,
				}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		check!(writer.into_inner() == std::fs::read(path).unwrap());
	}

	#[test]
	fn partition_mbr() {
		let path = "Reverse Engineering/Test Data/partition-mbr.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		check!(
			format
				== DeviceFormat {
					additional_options: Uuid::nil(),
					device: MBR_DEV
				}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		check!(writer.into_inner() == std::fs::read(path).unwrap());
	}

	#[test]
	fn file() {
		let path = "Reverse Engineering/Test Data/filedevice.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		check!(
			format
				== DeviceFormat {
					additional_options: Uuid::nil(),
					device: Device::File(File {
						device: Box::new(GPT_DEV),
						path: NullWideString::from(IMAGE_PATH),
					})
				}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		check!(writer.into_inner() == std::fs::read(path).unwrap());
	}

	#[test]
	fn ramdisk() {
		let path = "Reverse Engineering/Test Data/ramdisk.bin";

		let mut file = std::fs::File::open(path).unwrap();
		let format = DeviceFormat::read(&mut file).unwrap();

		check!(
			format
				== DeviceFormat {
					additional_options: Uuid::nil(),
					device: Device::Ramdisk(Ramdisk {
						file: File {
							device: Box::new(GPT_DEV),
							path: NullWideString::from(IMAGE_PATH),
						}
					})
				}
		);

		let mut writer = Cursor::new(vec![]);
		format.write(&mut writer).unwrap();
		check!(writer.into_inner() == std::fs::read(path).unwrap());
	}
}
