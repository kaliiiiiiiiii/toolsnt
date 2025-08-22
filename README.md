# Tools N't 2
#### A fork of [codeberg.org/erin/toolsnt](https://codeberg.org/erin/toolsnt), with improvements by [@kaliiiiiiiiii](https://github.com/kaliiiiiiiiii)

_Set of tools for messing with Windows NT_

Like…
- [bcdedit](bcdedit/): Windows Boot Configuration Data management tools
	- Made with reverse engineering (and love). No Microsoft employees were harmed in the process.
- [hivex](hivex/): (Hopefully) idiomatic Rust bindings to [Hivex] library for editing Windows Registry hives
- [wimlib](wimlib/): (Hopefully) idiomatic Rust bindings to the [wimlib] library for working with Windows images

[Hivex]: https://libguestfs.org/hivex.3.html 
[wimlib]: https://wimlib.net/

## Changes
See [all added commits](https://github.com/kaliiiiiiiiii/toolsnt/commits?author=kaliiiiiiiiii)
- added NOTICE.md
- improved wimlib for windows compability

## Windows build status
Building for windows currently fails

<details>

```bash
error[E0432]: unresolved import `std::os::fd`
   --> D:\data\projects\rinb\toolsnt\wimlib\src\wim\extract.rs:26:22
    |
26  |     std::{fs::File, os::fd::AsRawFd},
    |                         ^^ could not find `fd` in `os`
    |
note: found an item that was configured out
   --> C:\Users\aurin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib/rustlib/src/rust\library\std\src\os\mod.rs:186:9
    |
186 | pub mod fd;
    |         ^^
note: the item is gated here
   --> C:\Users\aurin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib/rustlib/src/rust\library\std\src\os\mod.rs:185:1
    |
185 | #[cfg(any(unix, target_os = "hermit", target_os = "trusty", target_os = "wasi", doc))]     
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^     

error[E0432]: unresolved import `std::os::fd`
   --> D:\data\projects\rinb\toolsnt\wimlib\src\wim\write.rs:10:22
    |
10  |     std::{fs::File, os::fd::AsRawFd},
    |                         ^^ could not find `fd` in `os`
    |
note: found an item that was configured out
   --> C:\Users\aurin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib/rustlib/src/rust\library\std\src\os\mod.rs:186:9
    |
186 | pub mod fd;
    |         ^^
note: the item is gated here
   --> C:\Users\aurin\.rustup\toolchains\stable-x86_64-pc-windows-msvc\lib/rustlib/src/rust\library\std\src\os\mod.rs:185:1
    |
185 | #[cfg(any(unix, target_os = "hermit", target_os = "trusty", target_os = "wasi", doc))]     
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^     

error[E0412]: cannot find type `timespec` in crate `sys`
   --> D:\data\projects\rinb\toolsnt\wimlib\src\wim\info.rs:808:36
    |
808 | fn convert_timedate(timespec: sys::timespec, high_secs: i32) -> OffsetDateTime {
    |                                    ^^^^^^^^ not found in `sys`

error[E0308]: mismatched types
  --> D:\data\projects\rinb\toolsnt\wimlib\src\string\mod.rs:42:3
   |
41 |     fn default() -> Self {
   |                     ---- expected `&TStr` because of return type
42 |         tstr!("")
   |         ^^^^^^^^^ expected `&TStr`, found `&U16CStr`
   |
   = note: expected reference `&TStr`
              found reference `&U16CStr`
   = note: this error originates in the macro `u16cstr` which comes from the expansion of the macro `tstr` (in Nightly builds, run with -Z macro-backtrace for more info)

error[E0599]: no method named `as_raw_fd` found for reference `&File` in the current scope       
   --> D:\data\projects\rinb\toolsnt\wimlib\src\wim\extract.rs:240:25
    |
240 |         let raw_handle = pipe.as_raw_fd();
    |                               ^^^^^^^^^
    |
help: there is a method `as_raw_handle` with a similar name
    |
240 -         let raw_handle = pipe.as_raw_fd();
240 +         let raw_handle = pipe.as_raw_handle();
    |

error[E0599]: no method named `as_raw_fd` found for reference `&File` in the current scope       
   --> D:\data\projects\rinb\toolsnt\wimlib\src\wim\extract.rs:266:25
    |
266 |         let raw_handle = pipe.as_raw_fd();
    |                               ^^^^^^^^^
    |
help: there is a method `as_raw_handle` with a similar name
    |
266 -         let raw_handle = pipe.as_raw_fd();
266 +         let raw_handle = pipe.as_raw_handle();
    |

error[E0599]: no method named `as_raw_fd` found for reference `&File` in the current scope
   --> D:\data\projects\rinb\toolsnt\wimlib\src\wim\write.rs:213:25
    |
213 |         let raw_handle = file.as_raw_fd();
    |                               ^^^^^^^^^
    |
help: there is a method `as_raw_handle` with a similar name
    |
213 -         let raw_handle = file.as_raw_fd();
213 +         let raw_handle = file.as_raw_handle();
    |

Some errors have detailed explanations: E0308, E0412, E0432, E0599.
For more information about an error, try `rustc --explain E0308`.
error: could not compile `wimlib` (lib) due to 7 previous errors
```

</details>


## Licence

This project is licensed under the [EUPL-1.2](./LICENSE.txt).

### Third-party software
This project depends on third-party crates under various licenses 
(including MIT, Apache-2.0, GPLv3, LGPL, BSD-3-Clause, ISC, Unicode, etc.).  
A complete list of licenses is available in [NOTICE.md](./NOTICE.md).

Especially take note of
- [NOTICE.md#wimlib](./NOTICE.md#wimlib)
- [NOTICE.md#LGPL 2.1](NOTICE.md#LGPL-2.1) for hivex
