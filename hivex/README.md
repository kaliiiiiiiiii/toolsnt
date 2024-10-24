# Hivex

_FFI bindings for the [Hivex] C library_

Hivex is a library for reading and manipulating Windows NT registry hives. This crate attempts to idiomatically wrap this library for usage in Rust.

Most of the documentation, which isn't a Rust-specific concept like `SelectedNode` is taken and adapted from the [Hivex] documentation.

## Core concepts
- Hive: Windows Registry database file. These do not have to correspond to `HKEY`s in the tree. [Learn more in Microsoft docs][Hive].
- Node: That's what Microsoft calls keys. These contain key-value entries (ye I know, confusing).
- Value: Values associated to keys in nodes. They have several types.
- Handle: An opaque reference to an entity inside hive.
- Selected{Node,Value}: Convenience wrapper referencing hive and the entity which allows you to manipulate it.

## Implementation notes
- Hivex library itself doesn't support creation of new hives. This crate contains a pre-defined empty registry hive created on Windows NT 10.0
- Strings retrieved from the hive will get its NUL-terminator striped by the bindings

> [!WARNING]
> Not everything is yet tested. Some data may be saved or retrieved incorrectly and corrupt your system. Proceed with caution.

[Hivex]: https://libguestfs.org/hivex.3.html
[Hive]: https://learn.microsoft.com/en-us/windows/win32/sysinfo/registry-hives