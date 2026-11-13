# Loader and recovery-stub inspection

The actual `build_loader`, `build_stub` and release entrypoint now use
`fm1-tooling::build_image` for their existing inspection decisions. The compiler,
linker, filesystem and process graph remain explicit Python delivery adapters.
The original stub source, flags, link entry and lock file are unchanged.

Rust rejects loader sections outside RAM, empty/oversized loader binaries,
non-code stub sections, incorrect entry/prefix/signature, exhausted stub margin
and chip-ROM references. It pads the inspected stub with the same `0xFF` bytes,
extracts the existing version, hashes the result and preserves the original
summary text. The shared disassembly reader also serves the application guard.
No input image is executed or flashed by these operations.

The lock decision follows inspection and its summary output, as before. Python
then reads the lock. Rust compares the exact version/hash tokens and returns
the original warning, release failure or explicit update plan. Only the existing
`FM1_STUB_LOCK=update` mode writes that plan through the filesystem adapter;
write failures still precede the update message. This work never sets that mode
against the repository's real lock.

Release identity/filename/version parsing and SDK checksum warning policy also
run in Rust. The release parser preserves existing global state when no release
is supplied, including repeated entrypoint calls. The pinned SDK manifest stays
data in the adapter; SDK lookup and its native I/O failures stay there too.

`python tests/rust_build_images.py FROZEN_SOURCE` verifies all 440 frozen source
hashes and invokes the actual old and current builders with compiler delivery
doubles. It compares returned bytes, exact stdout/errors, compiler arguments and
order, disassembly files, loader packages and lock outcomes. It includes missing
binary delivery, compound failures, margin/size boundaries, release spellings,
saved prior metadata and SDK warning order. No vendor compiler or hardware is
used by this differential test. Real target compilation remains a separate gate.

Section fields follow the vendor's ASCII index/hexadecimal format, including
ordinary signed/prefixed/underscored integer spellings used by the probes.
Checked host request limits retain the earlier 1,100,000-byte image cap and
signed 128-bit integer bound. Unicode release digits use the pinned regex
database; arbitrary future Python Unicode-version equivalence is not implied.
The valid firmware image/address ranges fit comfortably within these limits.

Application segment assembly, compiler/build graph execution and whole-release
physical qualification remain open. This port does not strengthen historical
missing-symbol defaults or change protected boot bytes.
