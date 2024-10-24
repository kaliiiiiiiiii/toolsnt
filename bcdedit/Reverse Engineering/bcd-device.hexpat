namespace bcd_device {
	import std.string;
	import std.mem;
	import std.io;
	import type.guid;
	import type.magic;

	// Utility //

	namespace utils {
		fn format_string(ref auto string) {
			return string.data;
		};

		struct NullString16 {
			char16 data[while(std::mem::read_unsigned($, sizeof(char16)) != 0)];
			char16 null_terminator;
		} [[
			sealed,
			transform("bcd_device::utils::format_string"),
			format   ("bcd_device::utils::format_string")
		]];

		struct MagicValue<T, auto Expected> {
			u64 pre_loc = $;
			T value;
						
			std::assert(
				value == Expected,
				std::format(
				    "Invalid magic! Expected `{}`, got `{}` at position 0x{:X}.",
				    Expected, value, $ - pre_loc
				)
			);
		};
	}

	// Partition //

	struct GptPartition {
		type::GUID partition;
		utils::MagicValue<u64, 0>;
		type::GUID disk;
	};

	struct MbrPartition {
		padding[2];
		utils::MagicValue<u8, 0x10>;
		padding[17];
		u32 partition;
		u32 disk;
	};

	struct Partition {
		u32 mbr_part  = $[$+ 20];
		u8  mbr_magic = $[$+ 2];
		
		if (mbr_part != 0 && mbr_magic == 0x10) {
		    MbrPartition partition;
		} else {
		    GptPartition partition;
		}
		
		padding[16];
	};

	/// File
	using Device;
	struct File {
		// ; 0x01; 0x6C; 0x05; 0x06 – Unknown meaning
		u32; u32 ; u32 ; u32 ; u32;
		padding[4];
		
		Device              device;
		utils::NullString16 path;
	};

	/// Ramdisk
	struct Ramdisk {
		u8 _unknown[20];
		File file;
	};
	
	/// LocateEx
	struct LocateEx {
	   u8 _unknown[62];
	   Device device;
	};

	// Device //

	enum Type : u64 {
		Partition = 0x48,
		File      = 0x80,
		Ramdisk   = 0x94,
		LocateEx  = 0xC6,
	};

	struct Device {
		Type type;
		
		match (type) {
		    (Type::Partition): Partition part;
		    (Type::File):      File file;
		    (Type::Ramdisk):   Ramdisk ramdisk;
		    (Type::LocateEx):  LocateEx locate_ex;
		}
	};
}

bcd_device::Device dev @ 0x18;
