# Fonts, patterns and sine assets (#22 checkpoint)

`gen_font.py`, `gen_pats.py` and the `gen_sint.c` compatibility command now use
the safe Rust host crate `fm1-tooling` for their generation policy. The emitted
headers retain the frozen generator layout and bytes on each qualified host.
No firmware/runtime path, device, server or build emitter changes here.

## Data and boundaries

The editable inputs are `tools/data/fontsets.json` and `patterns.json`, extracted
from the original Python literals without importing the pattern generator.
They retain the five font recipes and ten pieces, six patterns and six lanes.
Python exposes the original tuple/list shapes, names and default Chakra recipe.
The native `tools patterns` command embeds the same pattern input file.

```text
BDF text --------------------------> Rust cell parsing and clipping
font recipe -> Rust search/axes ----> Python Path / Pillow / FreeType
                         gray bytes -> Rust rounding/quantization/upscaling
                                      -> nibble rows, offsets and C declarations
                                      -> Python write_text / console
16-step grids -> Rust validation ----> LSB-first lane words and C declarations
1025 phases -> existing engine sine -> decimal formatting and C declarations
```

`fonts::Font` contains a height and owned `Glyph` values: signed advance, bitmap
width and row-major alpha bytes. BDF glyphs occupy a `BTreeMap` keyed by codepoint;
duplicate encodings replace the prior cell. The parser retains cell clipping,
DWIDTH, baseline offsets, eager `?` fallback, synthesized U-with-dots and missing
ASCII warnings. Signed arbitrary-width bitmap rows are inspected without a
fixed integer narrowing. Each output row packs its first pixel into the high
nibble, and an odd final pixel leaves the low nibble zero. Offsets remain u16.

For TTF files Rust selects search paths, variation axes/defaults, tabular-number
features, crop coordinates, ties-to-even advances, alpha thresholds and nearest
neighbor scaling. Python performs actual font opening/rasterization and file or
console I/O through synchronous adapter actions. No Pillow object or borrowed
pixel buffer is retained by Rust; pixels cross as owned framed JSON bytes.
Printing and final file writes retain their original order and partial output
on exceptions. `upscale(..., scale=1)` retains Python object identity.

The sine command calls `fm1_engine::effects::math::sin` with the original f32
phase association; it does not introduce another polynomial or use libm sine.
Its scientific formatting preserves nine decimal places and signed, two-digit
exponents. The frozen Windows MinGW C runtime rounds an exact decimal halfway
away from zero, while Linux libc/Rust round to even. At point 889 the same f32
`0xbf3ca000` therefore prints `-7.368164063e-01f` on Windows and
`-7.368164062e-01f` on Linux. An integer rational midpoint check preserves this
formatting distinction without changing the engine sample. The initial Windows
mismatch and raw outputs are retained under ignored `build/assets/`.

## Commands and bootstrap

```sh
cargo +1.94.1 build --release --locked -p fm1-cli
target/release/fm1-rs tools sine > sine.h
target/release/fm1-rs tools patterns > patterns.h
python3 tools/gen_font.py font.h
python3 tools/gen_pats.py
python3 tools/gen_sint.py > sine.h
gcc -O0 -I firmware/src/dsp tools/gen_sint.c -o /tmp/gs
/tmp/gs > sine.h
```

The existing font and pattern Python APIs and CLI arguments remain. The pattern
script still writes `firmware/src/dsp/pats.h` when imported. The C sine command
is a process launcher; from another working directory set `FM1_GEN_SINT_SCRIPT`
to the absolute Python script path. `FM1_PYTHON` selects its interpreter.

The existing binding lazily builds the native host CLI with Rust 1.94.1 in an
OS-specific target directory. `FM1_TOOLING_CLI` (or the existing
`FM1_CODEC_CLI` fallback) can select a prebuilt binary. Cargo, Rust 1.94.1 and
Pillow are therefore generator prerequisites. This is deliberately a native
host graph: it does not enable `fm1-sim/c-app`, invoke font generation from its
own build, or introduce tooling/SHA dependencies to the MCU or codec-WASM graph.
An unconfigured Python invocation was also exercised through the lazy build.
Build.py/emitter ownership remains with the root integration work.

Explicit limits cover otherwise unbounded caller-supplied data: 32 MiB frames,
16 MiB BDF text, 4,096 stored glyphs, at most 4,097 requested codepoints, dimensions
up to 2,048, integer scale 0..16, and 1,100,000 stored or output pixels per font.
Font integers are bounded to magnitude 2^40, scaled BDF advances to 32,768;
TTF advances are finite and at most 2,048 before scaling. Variation values must
be finite. A request admits at most 64 recipes, 128-byte labels and 4,096-byte
recipe filenames. The original packed-data `<65536` assertion is retained. These limits
are explicit refusals beyond the qualified domain, rather than claims that all
arbitrary Python objects or malformed/unbounded fonts retain behavior.

## Proof and remaining scope

`bash tests/rust_assets.sh FROZEN_SOURCE` builds the native CLI and executes
`assets_differential.py`; set `PYTHON` for a Pillow-enabled interpreter. It needs
Python, Pillow and GCC (or a compatible executable named by `CC`). Linux CI can
use the existing Python 3.10 / Pillow 12.3.0 authoring job; no NumPy, MIDI, USB,
audio device, C app or OS window is used.

The suite verifies all 440 immutable R01 source hashes before running exact
header/log comparisons for the repository's Terminus, Barlow and Chakra fonts.
Inter and Rajdhani have no font files in this source tree: their recipes, axes,
geometry, bytes and call ordering are tested against a deterministic fake
rasterizer, not presented as FreeType qualification of absent files. Other
cases cover random clipped BDF cells, signed and large bitmap words, malformed
input errors, missing characters, U fallback, negative advances, reverse ranges,
scale-one identity, ties-to-even advances, all gray levels, odd nibble tails,
offset overflow, partial output and write failures. Four hundred random grids
and invalid grids exercise lane packing. Compiled frozen C produces all 1,025
sine strings for comparison with native Rust, Python and both C launcher paths.
All 653 cases pass on Linux (Python 3.10.12, Pillow 12.3.0) and Windows
(Python 3.8.3, Pillow 10.4.0). Repeated Linux runs produce the same normalized
trace hash. The workspace's 159 tests, formatting and strict Clippy pass; the
prior 3,365 installer/rescue/telemetry cases and fake installer/selftest still pass.

`asset-generators-proof.json` records the exact source and host evidence. The
MCU DSP and its public WASM export surface are unchanged. Whole ticket #22
remains open for the root-owned build/package/LZ4/emitter work and subsequent
parameter, manual/site and acquisition/bootstrap tool policy. This checkpoint
does not claim physical device or unavailable-font measurements.
