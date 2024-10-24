use {
	super::{
		define_elements,
		format::{enum_formats, Device, Guid, GuidList, IntegerList, String},
	},
	crate::object::application::Application,
};

enum_formats! {
	pub enum BootUxPolicyValue {
		Disabled = 0,
		Basic    = 1,
		Standard = 2,
	}

	pub enum BootMenuPolicyValue {
		Legacy   = 0,
		Standard = 1,
	}
}

pub mod bootmgr {
	use {
		super::*,
		crate::{
			object::application::app_type::{BootMgr, FwBootMgr},
			typesystem::SubclassOf,
		},
	};

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	#[doc(hidden)]
	pub struct EitherBootmgr;
	impl SubclassOf<EitherBootmgr> for FwBootMgr {
		fn upcast(self) -> EitherBootmgr {
			EitherBootmgr
		}

		fn downcast_from(_: EitherBootmgr) -> Result<Self, EitherBootmgr>
		where
			Self: Sized,
		{
			Ok(FwBootMgr)
		}
	}

	impl SubclassOf<EitherBootmgr> for BootMgr {
		fn upcast(self) -> EitherBootmgr {
			EitherBootmgr
		}

		fn downcast_from(_: EitherBootmgr) -> Result<Self, EitherBootmgr>
		where
			Self: Sized,
		{
			Ok(BootMgr)
		}
	}

	define_elements! {
		for Application<EitherBootmgr>;

		DisplayOrder              : GuidList    = "0x24000001";
		BootSequence              : GuidList    = "0x24000002";
		DefaultObject             : Guid        = "0x23000003";
		Timeout                   : u64         = "0x25000004";
		AttemptResume             : bool        = "0x26000005";
		ResumeObject              : Guid        = "0x23000006";
		StartupSequence           : GuidList    = "0x24000007";
		ToolsDisplayOrder         : GuidList    = "0x24000010";
		DisplayBootMenu           : bool        = "0x26000020";
		NoErrorDisplay            : bool        = "0x26000021";
		BcdDevice                 : Device      = "0x21000022";
		BcdFilePath               : String      = "0x22000023";
		HormEnabled               : bool        = "0x26000024";
		HiberRoot                 : bool        = "0x26000025";
		PasswordOverride          : String      = "0x22000026";
		PinpassPhraseOverride     : String      = "0x22000027";
		ProcessCustomActionsFirst : bool        = "0x26000028";
		Unknown0x2600002A         : bool        = "0x2600002A";
		CustomActionsList         : IntegerList = "0x27000030";
		PersistBootSequence       : bool        = "0x26000031";
		SkipStartupSequence       : bool        = "0x26000032";
	}

	pub mod fve {
		use super::*;

		define_elements! {
			for Application<EitherBootmgr>;

			RecoveryUrl            : String      = "0x22000040";
			RecoveryMessage        : String      = "0x22000041";
			FlightBootMgr          : bool        = "0x26000042";
			UnlockRetryCountV4     : u64         = "0x25000043";
			UnlockRetryCountV6     : u64         = "0x25000044";
			ServerAddressV4        : String      = "0x22000045";
			ServerAddressV6        : String      = "0x22000046";
			StationAddressV4       : String      = "0x22000047";
			StationAddressV6       : String      = "0x22000048";
			StationSubnetMaskV4    : String      = "0x22000049";
			StationPrefixV6        : String      = "0x2200004A";
			GatewayV4              : String      = "0x2200004B";
			GatewayV6              : String      = "0x2200004C";
			Timeout                : u64         = "0x2500004D";
			RemotePortV4           : u64         = "0x2500004E";
			RemotePortV6           : u64         = "0x2500004F";
			StationPortV4          : u64         = "0x25000050";
			StationPortV6          : u64         = "0x25000051";
		}
	}
}
pub mod osloader {
	use {super::*, crate::object::application::app_type::OsLoader};

	enum_formats! {
		pub enum NxPolicyValue {
			OptIn     = 0,
			OptOut    = 1,
			AlwaysOff = 2,
			AlwaysOn  = 3,
		}

		pub enum PaePolicyValue {
			Default      = 0,
			ForceEnable  = 1,
			ForceDisable = 2,
		}

		// ↓ Ludicrous ↑

		pub enum TpmBootEntropyPolicyValue {
			Default      = 0,
			ForceDisable = 1,
			ForceEnable  = 2,
		}

		pub enum X2ApicPolicyValue {
			Default = 0,
			Dsiable = 1,
			Enable  = 2,
		}

		pub enum ForceOffPolicy {
			Default =      0,
			ForceDisable = 1,
		}

		pub enum SafeBootValue {
			Minimal  = 0,
			Network  = 1,
			DsRepair = 2,
		}

		pub enum TscSyncPolicyValue {
			Default  = 0,
			Legacy   = 1,
			Enhanced = 2,
		}

		pub enum DriverLoadFailurePolicyValue {
			Fatal           = 0,
			UseErrorControl = 1,
		}

		pub enum BootStatusPolicyValue {
			DisplayAllFailures           = 0,
			IgnoreAllFailures            = 1,
			IgnoreShutdownFailures       = 2,
			IgnoreBootFailures           = 3,
			IgnoreCheckpointFailures     = 4,
			DisplayShutdownFailures      = 5,
			DisplayCheckpointFailures    = 6,
			AlwaysDisplayStartupFailures = 7,
		}


		pub enum DbgTypeValue {
			Serial = 0,
			_1394  = 1,
			None   = 2,
			Net    = 3,
		}

		pub enum LaunchTypeValue {
			Off  = 0,
			Auto = 1,
		}
	}

	define_elements! {
		for Application<OsLoader>;

		OsDevice                    : Device                       = "0x21000001";
		SystemRoot                  : String                       = "0x22000002";
		AssociatedResumeObject      : Guid                         = "0x23000003";
		StampDisks                  : bool                         = "0x26000004";
		Unknown0x21000005           : Device                       = "0x21000005";
		Unknown0x25000008           : u64                          = "0x25000008";
		DetectKernelAndHal          : bool                         = "0x26000010";
		KernelPath                  : String                       = "0x22000011";
		HalPath                     : String                       = "0x22000012";
		DbgTransportPath            : String                       = "0x22000013";
		NxPolicy                    : NxPolicyValue                = "0x25000020";
		PaePolicy                   : PaePolicyValue               = "0x25000021";
		WinPeMode                   : bool                         = "0x26000022";
		DisableCrashAutoReboot      : bool                         = "0x26000024";
		UseLastGoodSettings         : bool                         = "0x26000025";
		DisableIntegrityChecks      : bool                         = "0x26000026";
		AllowPrereleaseSignatures   : bool                         = "0x26000027";
		NoLowMemory                 : bool                         = "0x26000030";
		RemoveMemory                : u64                          = "0x25000031";
		IncreaseUserVa              : u64                          = "0x25000032";
		PerformaceDataMemory        : u64                          = "0x25000033";
		UseVgaDriver                : bool                         = "0x26000040";
		DisableBootDisplay          : bool                         = "0x26000041";
		DisableVesaBios             : bool                         = "0x26000042";
		DisableVgaMode              : bool                         = "0x26000043";
		ClusterModeAddressing       : u64                          = "0x25000050";
		UsePhysicalDestination      : bool                         = "0x26000051";
		RestrictApicCluster         : u64                          = "0x25000052";
		EvStore                     : String                       = "0x22000053";
		UseLegacyApicMode           : bool                         = "0x26000054";
		X2ApicPolicy                : X2ApicPolicyValue            = "0x25000055";
		UseBootProcessorOnly        : bool                         = "0x26000060";
		NumberOfProcessors          : u64                          = "0x25000061";
		ForceMaximumProcessors      : bool                         = "0x26000062";
		ProcessorConfigurationFlags : u64                          = "0x25000063";
		MaximizeGroupsCreated       : bool                         = "0x26000064";
		ForceGroupAwareness         : bool                         = "0x26000065";
		GroupSize                   : u64                          = "0x25000066";
		UseFirmwarePciSettings      : bool                         = "0x26000070";
		MsiPolicy                   : ForceOffPolicy               = "0x25000071";
		PciExpressPolicy            : ForceOffPolicy               = "0x25000072";
		SafeBoot                    : SafeBootValue                = "0x25000080";
		SafeBootAlternativeShell    : bool                         = "0x26000081";
		BootLogInitialization       : bool                         = "0x26000090";
		VerboseObjectLoadMode       : bool                         = "0x26000091";
		KernelDebuggerEnabled       : bool                         = "0x260000A0";
		DebuggerHalBreakpoint       : bool                         = "0x260000A1";
		UsePlatformClock            : bool                         = "0x260000A2";
		ForceLegacyPlatform         : bool                         = "0x260000A3";
		UsePlatformTick             : bool                         = "0x260000A4";
		DisableDynamicDick          : bool                         = "0x260000A5";
		TscSyncPolicy               : TscSyncPolicyValue           = "0x250000A6";
		EmsEnabled                  : bool                         = "0x260000B0";
		ForceFailure                : u64                          = "0x250000C0";
		DriverLoadFailurePolicy     : DriverLoadFailurePolicyValue = "0x250000C1";
		BootMenuPolicy              : BootMenuPolicyValue          = "0x250000C2";
		AdvancedOptionsOneTime      : bool                         = "0x260000C3";
		EditOptionsOneTime          : bool                         = "0x260000C4";
		BootStatusPolicy            : BootStatusPolicyValue        = "0x250000E0";
		DisableElamDrivers          : bool                         = "0x260000E1";
		BootUxPolicy                : BootUxPolicyValue            = "0x250000F7";
		TpmBootEntropyPolicy        : TpmBootEntropyPolicyValue    = "0x25000100";
		XSavePolicy                 : u64                          = "0x25000120";
		XSaveAddFeature0            : u64                          = "0x25000121";
		XSaveAddFeature1            : u64                          = "0x25000122";
		XSaveAddFeature2            : u64                          = "0x25000123";
		XSaveAddFeature3            : u64                          = "0x25000124";
		XSaveAddFeature4            : u64                          = "0x25000125";
		XSaveAddFeature5            : u64                          = "0x25000126";
		XSaveAddFeature6            : u64                          = "0x25000127";
		XSaveAddFeature7            : u64                          = "0x25000128";
		XSaveRemoveFeature          : u64                          = "0x25000129";
		XSaveProcessorsMask         : u64                          = "0x2500012A";
		XSaveDisable                : u64                          = "0x2500012B";
		FveClaimedDeviceLockCounter : u64                          = "0x25000130";
		ImcHiveName                 : String                       = "0x22000137";
		ImcDevice                   : Device                       = "0x21000138";
		ManufacturingMode           : String                       = "0x22000140";
		EventLoggingEnabled         : bool                         = "0x26000141";
		VsmLaunchType               : LaunchTypeValue              = "0x25000142";
		DtraceEnabled               : bool                         = "0x26000145";
		SystemDataDevice            : Device                       = "0x21000150";
		OsArcDevice                 : Device                       = "0x21000151";
		Unknown0x21000152           : Device                       = "0x21000152";
		OsDataDevice                : Device                       = "0x21000153";
		BspDevice                   : Device                       = "0x21000154";
		BspFilePath                 : String                       = "0x21000155";
	}

	pub mod kernel_debugger {
		use super::*;

		define_elements! {
			for Application<OsLoader>;

			Type              : DbgTypeValue = "0x2500012C";
			BusParameters     : String       = "0x2200012D";
			PortAddress       : u64          = "0x2500012E";
			PortNumber        : u64          = "0x2500012F";
			Channel1394       : u64          = "0x25000131";
			UsbTargetName     : String       = "0x22000132";
			NetHostIp         : u64          = "0x25000133";
			NetHostPort       : u64          = "0x25000134";
			NetDhcp           : bool         = "0x26000135";
			NetKey            : String       = "0x22000136";
			Baudrate          : u64          = "0x25000139";
			NetHostIpV6       : u64          = "0x22000156";
			NetHostPortV6     : u64          = "0x22000161";
		}
	}

	pub mod hypervisor {
		pub use super::*;

		enum_formats! {
			pub enum IommuPolicyValue {
				Default = 0,
				Enable  = 1,
				Disable = 2,
			}

			pub enum DisableEnableValue {
				Disable = 0,
				Enable  = 1,
			}

			pub enum SchedulerTypeValue {
				Classic = 0,
				Core    = 1,
				Root    = 2,
			}

			pub enum PerfMonValue {
				System     = 0,
				Hypervisor = 1,
			}

			pub enum EnforcedCodeIntegrityValue {
				Disable = 0,
				Enable  = 1,
				Strict  = 2,
			}
		}

		define_elements! {
			for Application<OsLoader>;

			LaunchType            : LaunchTypeValue            = "0x250000F0";
			Path                  : String                     = "0x220000F1";
			DbgEnabled            : bool                       = "0x260000F2";
			DbgType               : DbgTypeValue               = "0x250000F3";
			DbgPortNumber         : u64                        = "0x250000F4";
			DbgBaudrate           : u64                        = "0x250000F5";
			DbgChannel1394        : u64                        = "0x250000F6";
			SlatDisabled          : bool                       = "0x260000F8";
			DbgBusParams          : String                     = "0x220000F9";
			NumProc               : u64                        = "0x250000FA";
			RootProcPerNode       : u64                        = "0x250000FB";
			UserLargeVtlb         : bool                       = "0x260000FC";
			DbgNetHostIp          : u64                        = "0x250000FD";
			DbgNetHostPort        : u64                        = "0x250000FE";
			DbgPages              : u64                        = "0x250000FF";
			DbgNetKey             : String                     = "0x22000110";
			ProcutSkuType         : String                     = "0x22000112";
			RootProc              : u64                        = "0x25000113";
			DbgNetDhcp            : bool                       = "0x26000114";
			IommuPolicy           : IommuPolicyValue           = "0x25000115";
			UseVApic              : bool                       = "0x26000116";
			LoadOptions           : String                     = "0x22000117";
			MsrFilterPolicy       : DisableEnableValue         = "0x25000118";
			MmioNxPolicy          : DisableEnableValue         = "0x25000119";
			SchedulerType         : SchedulerTypeValue         = "0x2500011A";
			RootProcNumaCores     : String                     = "0x2200011B";
			PerfMon               : PerfMonValue               = "0x2500011C";
			RootProcPerCode       : u64                        = "0x2500011D";
			RootProcNumaNodeLps   : String                     = "0x2200011E";
			EnforcedCodeIntegrity : EnforcedCodeIntegrityValue = "0x25000144";
		}
	}
}

pub mod resume {
	use {super::*, crate::object::application::app_type::Resume};

	define_elements! {
		for Application<Resume>;

		HiberFileDevice    : Device              = "0x21000001";
		HiberFilePath      : String              = "0x22000002";
		UseCustomSettings  : bool                = "0x26000003";
		X86PaeMode         : bool                = "0x26000004";
		AssociatedOsDevice : Device              = "0x21000005";
		DebugOptionEnabled : bool                = "0x26000006";
		BootUxPolicy       : BootUxPolicyValue   = "0x25000007";
		BootMenuPolicy     : BootMenuPolicyValue = "0x25000008";
		HromEnabled        : bool                = "0x26000024";
	}
}

pub mod memdiag {
	use {super::*, crate::object::application::app_type::MemDiag};

	enum_formats! {
		pub enum TestMixValue {
			Basic    = 0,
			Extended = 1,
		}

		pub enum TestToFailValue {
			Stride          = 0,
			Mats            = 1,
			InverseCoupling = 2,
			RandomPattern   = 3,
			Checkerboard    = 4,
		}
	}

	define_elements! {
		for Application<MemDiag>;

		PassCount          : u64             = "0x25000001";
		TestMix            : TestMixValue    = "0x25000002";
		FailureCount       : u64             = "0x25000003";
		CacheEnabled       : bool            = "0x26000003";
		TestToFail         : TestToFailValue = "0x25000004";
		FailuresEnabled    : bool            = "0x26000004";
		CacheEnable        : bool            = "0x26000005";
		StrideFailureCount : u64             = "0x25000005";
		InvcFailureCount   : u64             = "0x25000006";
		MatsFailureCount   : u64             = "0x25000007";
		RandFailureCount   : u64             = "0x25000008";
		ChckrFailureCount  : u64             = "0x25000009";
	}
}

pub mod ntldr_setupldr {
	use {
		super::*,
		crate::{
			object::application::app_type::{NtLdr, SetupLdr},
			typesystem::SubclassOf,
		},
	};

	#[derive(Clone, Copy, Debug, PartialEq, Eq)]
	#[doc(hidden)]
	pub struct NtOrSetupLdr;
	impl SubclassOf<NtOrSetupLdr> for NtLdr {
		fn upcast(self) -> NtOrSetupLdr {
			NtOrSetupLdr
		}

		fn downcast_from(_: NtOrSetupLdr) -> Result<Self, NtOrSetupLdr>
		where
			Self: Sized,
		{
			Ok(NtLdr)
		}
	}

	impl SubclassOf<NtOrSetupLdr> for SetupLdr {
		fn upcast(self) -> NtOrSetupLdr {
			NtOrSetupLdr
		}

		fn downcast_from(_: NtOrSetupLdr) -> Result<Self, NtOrSetupLdr>
		where
			Self: Sized,
		{
			Ok(SetupLdr)
		}
	}

	define_elements! {
		for Application<NtOrSetupLdr>;

		BpbString: String = "0x22000001";
	}
}

pub mod startup {
	use {super::*, crate::object::application::app_type::Startup};

	define_elements! {
		for Application<Startup>;

		PxeSoftReboot      : bool   = "0x26000001";
		PxeApplicationName : String = "0x22000002";
	}
}

pub mod bootapp {
	use {super::*, crate::object::application::app_type::BootApp};

	define_elements! {
		for Application<BootApp>;

		EnableBootDebugPolicy  : bool = "0x26000145";
		EnableBootOrderClean   : bool = "0x26000146";
		EnableDeviceId         : bool = "0x26000147";
		EnableFfuLoader        : bool = "0x26000148";
		EnableIuLoader         : bool = "0x26000149";
		EnableMassStorage      : bool = "0x2600014A";
		EnableRpmbProvisioning : bool = "0x2600014B";
		EnableSecureBootPolicy : bool = "0x2600014C";
		EnableStartCharge      : bool = "0x2600014D";
		EnableResetTpm         : bool = "0x2600014E";
	}
}
