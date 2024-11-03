//! Well known object UUIDs
//! 
//! There UUIDs have a special meaning for Windows Boot Manager

use uuid::{uuid, Uuid};

/// Settings for Emergency Management Services
pub const EMS_SETTINGS_GROUP: Uuid = uuid!("0CE4991B-E6B3-4B16-B23C-5E0D9250E5D9");
/// Settings for loader of saved memory image (resume from hibernation)
pub const RESUME_LOADER_SETTINGS_GROUP: Uuid = uuid!("1AFA9C49-16AB-4A5C-4A90-212802DA9460");
/// Settings for NT Kernel debugger
pub const KERNEL_DEBUGGER_SETTINGS_GROUP: Uuid = uuid!("313E8EED-7098-4586-A9BF-309C61F8D449");
/// Settings for debugger
pub const DEBUGGER_SETTINGS_GROUP: Uuid = uuid!("4636856E-540F-4170-A130-A84776F4C654");
/// Legacy Windows Loader (NTLDR), used in version pre 6.0 (<Vista)
pub const WINDOWS_LEGACY_NTLDR: Uuid = uuid!("466F5A88-0AF2-4F76-9038-095B170DC21C");
/// Group with information got from memory tester
pub const BAD_MEMORY_GROUP: Uuid = uuid!("5189B25C-5558-4BF2-BCA4-289B11BD29E2");
/// Inherit settings for Boot Loader
pub const BOOT_LOADER_SETTINGS_GROUP: Uuid = uuid!("6EFB52BF-1766-41DB-A6B3-0EE5EFF72BD7");
/// Windows Setup for UEFI
pub const WINDOWS_SETUP_EFI: Uuid = uuid!("7254A080-1510-4E85-AC0F-E7FB3D444736");
/// Inherit settings for all
pub const GLOBAL_SETTINGS_GROUP: Uuid = uuid!("7EA2E1AC-2E61-4728-AAA3-896D9D0A9F0E");
/// Settings for Hyper-V
pub const HYPERVISOR_SETTINGS_GROUP: Uuid = uuid!("7FF607E0-4395-11DB-B0DE-0800200C9A66");
/// Windows Boot Manager
pub const WINDOWS_BOOTMGR: Uuid = uuid!("9DEA862C-5CDD-4E70-ACC1-F32B344D4795");
/// Template object for Windows on legacy boot (IBM PC AT)
pub const WINDOWS_OS_TARGET_TEMPLATE_PCAT: Uuid = uuid!("A1943BBC-EA85-487C-97C7-C9EDE908A38A");
/// FwBootMgr object
pub const FIRMWARE_BOOTMGR: Uuid = uuid!("A5A30FA2-3D06-4E9F-B5F4-A01DF9D1FCBA");
/// Options for ramdisk devices
pub const WINDOWS_SETUP_RAMDISK_OPTIONS: Uuid = uuid!("AE5534E0-A924-466C-B836-758539A3EE3A");
/// Template object for Windows on UEFI boot
pub const WINDOWS_OS_TARGET_TEMPLATE_EFI: Uuid = uuid!("B012B84D-C47C-4ED5-B722-C0C42163E569");
/// Windows Memory Tester
pub const WINDOWS_MEMORY_TESTER: Uuid = uuid!("B2721D73-1DB4-4C62-BF78-C548A880142D");
/// Windows Setup for lgacy boot (IBM PC AT)
pub const WINDOWS_SETUP_PCAT: Uuid = uuid!("CBD971BF-B7B8-4885-951A-FA03044F5D71");

/*
	== These are not actual object UUIDs. They are for Windows BCDEdit to resolve. ==

	pub const DEFAULT_BOOT_ENTRY: Uuid = uuid!("1CAE1EB7-A0DF-4D4D-9851-4860E34EF535");
	pub const CURRENT_BOOT_ENTRY: Uuid = uuid!("FA926493-6F1C-4193-A414-58F0B2456D1E");
*/
