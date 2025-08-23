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
See [all added commits](https://github.com/kaliiiiiiiiii/toolsnt/commits?author=kaliiiiiiiiii) or [changes comparison](https://github.com/kaliiiiiiiiii/toolsnt/compare/26978095e8069a5d32a31577d41854bd811c6724...kdl)
- added NOTICE.md
- build for windows (gnu or mvc) x64 on windows (`mingw-w64-clang-x86_64-clang`)

#### TODO
Support
- [ ] cross-build (build on linux for windows)
- [ ] build windows for
	- [ ] `CLANG32` (`mingw-w64-clang-i686-clang`)
	- [ ] `CLANGARM64`(`mingw-w64-clang-aarch64-clang`)

## Licence

This project is licensed under the [EUPL-1.2](./LICENSE.txt).

### Third-party software
This project depends on third-party crates under various licenses 
(including MIT, Apache-2.0, GPLv3, LGPL, BSD-3-Clause, ISC, Unicode, etc.).  
A complete list of licenses is available in [NOTICE.md](./NOTICE.md).

Especially take note of
- [NOTICE.md#wimlib](./NOTICE.md#wimlib)
- [NOTICE.md#LGPL 2.1](NOTICE.md#LGPL-2.1) for hivex
