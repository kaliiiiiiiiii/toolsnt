use {
	super::format::{Device, String},
	crate::object::device::Device as DeviceObj,
};

super::define_elements! {
	for DeviceObj;

	ImageOffset           : u64    = "0x35000001";
	TftpClientPort        : u64    = "0x35000002";
	SdiDevice             : Device = "0x31000003";
	SdiPath               : String = "0x32000004";
	ImageLength           : u64    = "0x35000005";
	ExportAsCd            : bool   = "0x36000006";
	TftpBlockSize         : u64    = "0x35000007";
	TftpWindowSize        : u64    = "0x35000008";
	MulticastEnabled      : bool   = "0x36000009";
	MulticastTftpFallback : bool   = "0x3600000A";
	TftpVarWindow         : bool   = "0x3600000B";
	VhdRamdiskBoot        : bool   = "0x3600000C";
	Unknown0x3500000D     : u64    = "0x3500000D";
}
