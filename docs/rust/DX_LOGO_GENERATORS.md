# DY voice and logo generators (#22 checkpoint)

The existing `gen_fm6_dy.py` and `make_logo.py` entry points execute their asset
selection and generation policy in safe `fm1-tooling` Rust. Python retains
filesystem effects, console encoding and Pillow's PNG encoder. The source
firmware, web assets, build emitter and physical device are unchanged.

## DX bank data and selection

`fm1_formats::voice_file::BulkBanks<'a>` is an allocation-free, `no_std`, borrowed
iterator over `[u8;4096]` payloads. Both the actual authoring `import_voice` path
and the DY generator use it. The three existing import contracts remain distinct:

| Reader | File shape | Validation |
| --- | --- | --- |
| `BulkBanks` | VMEM header at any byte offset; skip 4,104 bytes after acceptance | Requires six-byte header shape and complete extent; ignores channel, checksum, trailer and high data bits |
| `FileVoices` | Raw 4,096-byte bank or concatenated Yamaha bulk/single messages | Uses message boundaries and legacy file checks; ignores checksum |
| `voice::DxMessage` | A live Yamaha message | Strict message validation |

An incomplete `F0 43` candidate stops the header-only scan in the same place as
the original. Accepted payloads are not rescanned for nested headers. No policy
is silently tightened by calling the strict live decoder during file import.

The generator retains manifest field splitting, comments, negative bank/voice
indices, duplicate-slot replacement and contiguous `0..n-1` validation. Files
are read again for each manifest row, retaining the old I/O and failure order.
A `BTreeMap<i64, Voice>` stores selected 128-byte records with their labels and
source metadata. ROM comparison uses a `BTreeSet<[u8;118]>`: voice names do not
affect refusal. The ROM warning row is printed before refusal. Header and binary
writes follow all selection checks, in the original order.

The user audition bank keeps the first 32 U entries, appends the generator's
original INIT records if needed, and prints the original truncation message.
`tools/data/dy-init.json` is its canonical 128-byte input. This INIT differs from
the runtime `FM6_INIT`: byte 99 (operator-six level) is 0 in the generator and 99
in the runtime. A test prevents accidental substitution. Audition framing reuses
the shared checksum function while preserving the generator's acceptance of
high-bit file bytes; the strict live `dx7_bulk` API would reject those inputs.

Python's `INIT`, `dumps`, `name_of` and `main` APIs remain, including the nested
list-of-byte-record shape. `python -I tools/gen_fm6_dy.py ...` is supported: the
wrapper adds only its trusted sibling tools directory for the native binding.
Bank files are never Python imports. Open manifest readers close at EOF or on
an exception. Adapter exceptions preserve their original Python object/type.

## Logo data and pixels

`tools/data/logo-palette.json` preserves the eight original RGB tuples. Python's
mutable `PAL` API is passed to Rust on each render, so caller changes are used.
Rust parses the existing C mascot declaration and keeps its exact 23-by-23 shape
check. It returns an owned `Image { width, height, rgba }` with transparent black
background, nearest-neighbor pixels and alpha 255 for palette entries.

The optional canvas size retains centered paste and clipping, including floor
division for negative odd offsets. Scale zero and falsy size zero retain their
old behavior. The Python `render` API returns an RGBA Pillow image. `png` still
uses `save(..., optimize=True)`, preserving actual encoder bytes on each host.
Rust chooses the main command's sizes, output order, filenames, byte-length log,
base64 encoding and exact inline logo/favicon matches. A failed second write or
editor rewrite retains the same preceding files and printed output as R01.

## Native commands and bounds

```sh
cargo +1.94.1 build --release --locked -p fm1-cli
target/release/fm1-rs tools dy-build MANIFEST BANKS OUTROOT [ROMDIR]
target/release/fm1-rs tools logo-rgba firmware/src/app/ui.c 1 32 logo.rgba
python3 -I tools/gen_fm6_dy.py tools/fm6_dy.txt BANKS [--rom ROMDIR]
python3 tools/make_logo.py [--inline]
```

`dy-build` writes the same C header, packed binary and optional audition SysEx
below OUTROOT, using native Rust file I/O without Python. As before, the firmware
output directory must exist. `logo-rgba` emits raw RGBA plus JSON dimensions;
Pillow is needed only for the existing Python PNG command. Native commands use
UTF-8 text; retained Python commands preserve Python's encoding/errors.

The existing binding lazily builds a pinned Rust 1.94.1 host CLI, or accepts
`FM1_TOOLING_CLI` / `FM1_CODEC_CLI`. Its graph excludes the C-app simulator and
does not add a dependency crate. No public MCU/WASM ABI changes occur.

Explicit host limits admit 1,100,000-byte bank buffers, 4,096 manifest records,
16-KiB manifest rows, 4,096 ROM directory entries, 65,536 distinct ROM operators,
128 CLI arguments and integer indices of magnitude at most 2^40. Logo limits
are 16-MiB C source and inline output, 64 render rows/columns, 256 palette entries,
integer scale 0..16 and optional canvas size 0..512. Inline replacement checks
growth before appending, so repeated tags cannot multiply a small input into
an unbounded output. Larger inputs receive explicit refusals. These limits are
additional supported-domain constraints, not claims about arbitrary Python
objects or unlimited files.

## Qualification and remaining work

`bash tests/rust_dx_logo.sh FROZEN_SOURCE` needs Rust 1.94.1, Python with Pillow
and mido, and GCC for the prior asset gate. `PYTHON` can select a configured
interpreter. The runner uses a separate Linux Cargo target directory; Windows
proof uses `build/dx-logo/windows`, avoiding mixed-platform rlib reuse.

The new suite checks all 440 R01 source hashes and 621 exact cases: 137 bank
vectors, 150 name inputs, the real authoring consumer, manifest/refusal/file-order
paths, isolated Python and native CLIs, real PNG bytes, RGBA/crop edge cases,
mutable palettes, base64, complete HTML pages and partial write failures.
It replays the shipped 62-row manifest using all 30 frozen factory records at
their original bank/voice positions, reproducing the entire frozen `fm6_dy.h`
byte for byte. U-only source records unavailable in this tree use deterministic
fixtures; this is not a claim to possess or verify the original external banks.

The 903-case authoring differential gate and previous 653-case font/pattern/sine
gate also run against the new shared reader and CLI on Linux and Windows.
The full workspace's 164 tests, strict Clippy, formatting and `no_std` WASM check
pass; Windows also passes all 16 tooling tests.
Dedicated format tests distinguish all three reader profiles and verify borrowed
payload addresses. Dedicated logo tests cover admission and output amplification.
The machine-readable proof records exact sources, outputs, host identities and
workspace checks. No generated production files or device handles are touched.

Whole #22 remains open for root-owned build/emitter work and subsequent parameter,
manual/site and acquisition/bootstrap policy. This batch leaves the current UI
and `make_params.py` ownership unchanged.
