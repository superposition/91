# Build image and source guards

`tools/build.py::check` and `mmio_check` now call the host-only
`fm1-tooling` crate. Python supplies compiler output, image bytes and decoded
source files in native filesystem order. Rust decides which diagnostics to
return. Neither operation opens a device or executes a compiler.

The image gate preserves entry-address/stub-prefix checks, RAM-text call
rejection, forbidden chip-ROM references, application-slot and commit-sector
bounds, soft-double detection, and RAM/pool headroom. It preserves the old
diagnostic order and wording, including duplicate ROM-reference reports and
sorted/truncated symbol lists. Decimal and hexadecimal disassembly addresses
wrap to 32 bits as before. Missing resource symbols retain the old zero
fallback; this port does not strengthen that historical check.

The source gate strips comments while retaining line positions, builds the
transitive register-macro graph from HAL headers, and checks source/loader
lines for reserved addresses, volatile casts, assembly and register macros.
The empty compiler barrier and address-of-RAM-flag cast remain allowed.
Filesystem decoding, path spelling, sorted path order and file I/O remain in
the adapter so Windows diagnostics retain their existing separators and order.

The host dependency `regex` is pinned to 1.13.1; firmware/no-std crates do not
depend on it. Python-compatible word and whitespace classes and CRLF-aware
`splitlines` are explicit. Controlled Unicode cases include combining marks,
nonprinting characters and decimal-digit sets. The Unicode property database
comes from the pinned Rust regex dependency, rather than an arbitrary future
Python runtime. Vendor outputs are ASCII. The bridge accepts image arrays up
to 1,100,000 bytes and hexadecimal symbol values up to signed 128-bit range;
these checked request limits exceed every valid application image/address.

Run `python tests/rust_build_guards.py FROZEN_SOURCE`. The runner verifies all
440 frozen source hashes, then compares the actual old and current adapters:
605 image/disassembly/resource profiles and 322 source snapshots, including
both real source trees. Controlled Unicode source fixtures use identical UTF-8
decoding on both platforms; the two real-tree probes use native decoding.
No golden diagnostics are regenerated from the Rust implementation.

This checkpoint does not move compiler process orchestration, linker-section
inspection, SDK lookup or stub-lock management. Physical image execution and
stack/timing qualification remain separate work.
