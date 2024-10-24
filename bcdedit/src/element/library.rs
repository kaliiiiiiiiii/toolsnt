//! Library elements (applicable to all object classes)

use {
	super::{
		define_elements,
		format::{enum_formats, Device, GuidList, IntegerList, String},
	},
	crate::object,
};

enum_formats! {
	pub enum FirstMegabytePolicyValue {
		UseNone    = 0,
		UseAll     = 1,
		UsePrivate = 2,
	}

	pub enum ConfigAccessPolicyValue {
		Default          = 0,
		DisallowMmConfig = 1,
	}

	pub enum SiPolicyValue {
		Default = 0,
		Enable  = 1,
		Disable = 2,
	}

	pub enum GraphicsResolutionValue {
		R1024x768 = 0,
		R800x600  = 1,
		R1024x600 = 2,
	}

	pub enum BootErrorUxValue {
		Legacy   = 0,
		Standard = 1,
		Simple   = 2,
	}

	pub enum BootMeasurementLogFormatValue {
		Default = 0,
		Sha1    = 1,
	}

	pub enum DisplayRotationValue {
		Deg0   = 0,
		Deg90  = 1,
		Deg180 = 2,
		Deg270 = 3,
	}

	pub enum LinearAddress57PolicyValue {
		Default = 0,
		OptOut  = 1,
		OptIn   = 2,
	}
}

define_elements! {
	for object::Any;
	ApplicationDevice            : Device                        = "11000001";
	ApplicationPath              : String                        = "12000002";
	Description                  : String                        = "12000003";
	Locale                       : String                        = "12000004";
	Inherit                      : GuidList                      = "14000005";
	TruncatePhysicalMemory       : u64                           = "15000007";
	RecoverySequence             : GuidList                      = "14000008";
	AutoRecoveryEnabled          : bool                          = "16000009";
	BadMemoryList                : u64                           = "1700000A";
	AllowBadMemoryAccess         : bool                          = "1600000B";
	FirstMegabytePolicy          : FirstMegabytePolicyValue      = "1500000C";
	RelocatePhysicalMemory       : u64                           = "1500000D";
	AvoidLowPhysicalMemory       : u64                           = "1500000F";
	TraditionalKsegMappigns      : bool                          = "1600000F";
	EmsEnabled                   : bool                          = "15000020";
	EmsPort                      : u64                           = "15000022";
	EmsBaudrate                  : u64                           = "15000023";
	AttemptNonBcdStart           : bool                          = "16000031";
	DisplayAdvancedOptions       : bool                          = "16000040";
	DisplayOptionsEdit           : bool                          = "16000041";
	FveKeyRingAddress            : u64                           = "15000042";
	BsdLogDevice                 : Device                        = "11000043";
	BsdLogPath                   : String                        = "12000044";
	BsdPreserveLog               : bool                          = "16000045";
	GraphicsModeDisabled         : bool                          = "15000046";
	ConfigAccessPolicy           : ConfigAccessPolicyValue       = "15000047";
	DisableIntegrityChecks       : bool                          = "16000048";
	AllowPrereleaseSignatures    : bool                          = "16000049";
	FontPath                     : String                        = "1200004A";
	SiPolicy                     : SiPolicyValue                 = "1500004B";
	FveBandId                    : u64                           = "1500004C";
	ConsoleExtendedInput         : bool                          = "16000050";
	InitialConsoleInput          : u64                           = "15000051";
	GraphicsResolution           : GraphicsResolutionValue       = "15000052";
	RestartOnFailure             : bool                          = "16000053";
	GraphicsForceHighestMode     : bool                          = "16000054";
	IsolatedExecutionContext     : bool                          = "16000060";
	MultiBootSystem              : bool                          = "16000071";
	ForceNoKeyboard              : bool                          = "16000072";
	AliasWindowsKey              : bool                          = "16000073";
	BootShutdownDisabled         : bool                          = "16000074";
	PerformaceFrequency          : u64                           = "15000075";
	SecureBootRawPolicy          : u64                           = "15000076";
	AllowedInMemorySettings      : IntegerList                   = "17000077";
	Unknown0x16000078            : bool                          = "16000078";
	MobileGraphics               : bool                          = "1600007A";
	ForceFipsCrypto              : bool                          = "1600007B";
	BootErrorUx                  : BootErrorUxValue              = "1500007D";
	AllowFlightSignatures        : bool                          = "1600007E";
	BootMeasurementLogFormat     : BootMeasurementLogFormatValue = "1500007F";
	DisplayRotation              : DisplayRotationValue          = "15000080";
	LogControl                   : u64                           = "15000081";
	NoFirmwareSync               : bool                          = "16000082";
	Unknown0x11000083            : Device                        = "11000083";
	WindowsSystemDevice          : Device                        = "11000084";
	Unknown0x16000085            : bool                          = "16000085";
	Unknown0x15000086            : u64                           = "15000086";
	NumLockOn                    : bool                          = "16000087";
	AdditionalCiPolicy           : String                        = "12000088";
	LinearAddress57Policy        : LinearAddress57PolicyValue    = "15000088";

	OneshotSkipFfuUpdate         : bool                          = "26000202";
	ForceFfu                     : bool                          = "26000203";
	BootThreshold                : u64                           = "25000510";
	OffModeCharging              : bool                          = "26000512";
	Bootflow                     : u64                           = "25000AAA";
}

pub mod debugger {
	use super::*;

	enum_formats! {
		pub enum TypeValue {
			UseNone    = 0,
			UseAll     = 1,
			UsePrivate = 2,
		}

		pub enum StartPolicyValue {
			UseNone    = 0,
			UseAll     = 1,
			UsePrivate = 2,
		}
	}

	define_elements! {
		for object::Any;

		Enabled         : bool             = "16000010";
		Type            : TypeValue        = "15000011";
		PortAddress     : u64              = "15000012";
		PortNumber      : u64              = "15000013";
		Baudrate        : u64              = "15000014";
		Channel1394     : u64              = "15000015";
		UsbTargetName   : String           = "12000016";
		NoUMEx          : bool             = "16000017";
		StartPolicy     : StartPolicyValue = "15000018";
		BusParameters   : String           = "12000019";
		NetHostIp       : u64              = "1500001A";
		NetPort         : u64              = "1500001B";
		NetDhcp         : bool             = "1600001C";
		NetKey          : String           = "1200001D";
		NetVm           : bool             = "1600001E";
		NetHostIpv6     : String           = "1200001F";
	}
}

pub mod boot_ux {
	use super::*;

	enum_formats! {
		pub enum DisplayMessageValue {
			Default             = 0,
			Resume              = 1,
			HyperV              = 2,
			Recovery            = 3,
			StartupRepair       = 4,
			SystemImageRecovery = 5,
			CommandPrompt       = 6,
			SystemRestore       = 7,
			PushButtonReset     = 8,
		}
	}

	define_elements! {
		for object::Any;

		DisplayMessage         : DisplayMessageValue = "15000065";
		DisplayMessageOverride : DisplayMessageValue = "15000066";
		LogoDisable            : bool                = "16000067";
		TextDisable            : bool                = "16000068";
		ProgressDisable        : bool                = "16000069";
		FadeDisable            : bool                = "1600006A";
		ReservePoolDebug       : bool                = "1600006B";
		Disable                : bool                = "1600006C";
		FadeFrames             : u64                 = "1500006D";
		DumpStats              : bool                = "1600006E";
		ShowStats              : bool                = "1600006F";
		TransitionTime         : u64                 = "16000079";
	}
}
