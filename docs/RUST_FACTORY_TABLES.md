# Compact factory musical data

This implements the musical-data scope of [R04 / #5](https://github.com/superposition/91/issues/5). The Rust emulator now reads factory recipes, patterns, modifiers, parameter defaults, musical names and packed lines through maintained Rust data and accessors. The independent C implementation remains selectable. Voice synthesis, harmonic decisions and effects retain their own migration tickets below.

## Representation and execution

`fm1-engine::factory` computes the 19 × 10 factory pieces from the original compact representation: 15 donor recipes, 19 genre overrides, shared pattern masks, palettes, and tempo/key/mode rows. It does not retain a 190 × 176 preset lookup table. The expanded JSON export is test evidence only; none of the JSON fixtures is included by production Rust code.

| Original data | Maintained Rust representation | Real emulator consumer |
| --- | --- | --- |
| `XBASE`, `X_*`, `GEN_OVR`, `GO_*` | Checked `ParamId` / signed `i16` override pairs | `en_piece_g` fills the caller's parameters and scale pool |
| `XROW`, `XPAT`, `PSY_DOWN`, `GEN_DONOR` | Compact byte/word tables and checked `GenreId` / `PieceId` | Factory piece/pattern/modifier loading |
| `GEN_TR`, `GEN_PLUCK`, `PALETTE`, `PLUCK_RATE`, `PAD_K` | Original compact rows; procedural donor/genre variation | Factory sound, key, tempo and modifier selection |
| `PARAMS`, `EN_MODE_POOL` | Existing 176 typed parameter rows and eight scale masks | Parameter inspection, initial defaults and settings application |
| 26 named arrays, including 190 piece names | Static NUL strings; immutable C-shaped pointer arrays | Existing firmware screen, parameter formatting and voice labels |
| `NAIWAN_ARC`, `NAIWAN_KIND`, `NAIWAN_BARS` | Ten exact packed six-byte progression / three-byte kind pairs | `naiwan_prog` posts the original progression commands |
| `EN_PIECE_PATS[0]` | 72-byte `PatternBank` | Initial patterns copied to the existing volatile C bank |
| Acid/pluck line packing | 18-byte `AcidLine`, 12-byte `PluckLine` | Stored-piece load and Global Save; original signed command words |

All 15 donor rows are maintained and checked, including donors not selected by the current genre map. The original `RETRO` intermediate pad lookup is omitted because the same C function unconditionally overwrites it with the final donor palette value. The C `pats.h` rows 1–9 are historical data with no current factory caller: the factory API uses `XPAT`; only boot row 0 is live. They remain available in the C reference.

`GenreId`, `PieceId`, `PatternId`, `LaneId` and `StepId` reject invalid indices. `AcidStep` retains its nine-bit offset/octave/gate/accent/slide payload; `PluckStep` retains all six wire bits, including rungs 12–15 even where playback clamps the ladder. `Progression` and `ChordKinds` check each field before writing and preserve reserved bits. Patterns remain exactly 72 bytes; no enum renumbering, storage magic, bank layout, signed command payload or WASM export is changed.

`EN_RUST_FACTORY=1` selects the new path in the existing Rust-enabled emulator build. `EN_RUST_FACTORY=0` keeps the C factory path with the other Rust features enabled. `EN_RUST_JAM=0` selects the all-C reference. Hardware builds continue to use C unless explicitly configured otherwise.

## Ownership and safety

Both `fm1-types` and `fm1-engine` forbid unsafe code and use no allocator. Generation returns fixed-size values: a preset is 354 bytes, a pattern bank 72 bytes and modifiers 96 bytes. These are caller/local values, not a global expanded cache. Factory generation consumes no RNG, and initialization preserves the C pending-progression side effect and volatile pattern writes.

The small bridge validates index, enum, null and capacity arguments before writes; pointer validity/disjointness remains the documented C caller obligation. Payloads are copied into C-owned storage. The line codec accepts unaligned word input/output using bounded unaligned reads/writes. Immutable string/parameter views have static lifetimes; two narrowly documented `Sync` implementations cover only immutable raw-pointer descriptors. Loaded user-voice labels still use the existing mutable C array, initialized once from the Rust factory labels. Subsequent engine initialization preserves DY selection and loaded user labels/data, matching the original static C initialization; a frozen-C/native bridge regression checks this reset path.

Diagnostic indices 32–35 count table accesses, codec calls, rejected inputs and copied bytes. They remain developer telemetry through the existing `web_rust_stat` interface and add no browser exports or public status panels.

## Independent evidence

The R01 extraction is immutable and verified against all 440 entries in `tests/reference/source.sha256` before exporting. `tests/reference/factory_reference.c` is compiled against that C source, not the Rust implementation. Its three hash-pinned fixtures contain:

- Every one of the 190 × 176 signed parameter values, all pools, 190 × 36 pattern words and 190 × 96 modifier bytes.
- All 176 parameter names/ranges/defaults; 26 name groups (581 entries), eight mode masks, 19 donor IDs, boot patterns and ten progression/kind payloads.
- Every compact recipe/table entry; 512 acid and 64 pluck cross-byte vectors; 166 valid progression/kind field edits over nonzero reserved bits.

The test recompiles the current selectable C path and the actual C→Rust bridge and requires both complete exports to match the frozen JSON byte for byte. It separately checks every acid/pluck command emitted by C after Rust decode: 1,152 codec calls, 417 table accesses, 30,258 copied bytes, zero rejections. Invalid FFI indices/capacities/nulls preserve output sentinels; checked type tests cover invalid fields, full/empty masks, cross-byte fields and exact sizes. The generator is reproducible after `cargo fmt` and is never executed at runtime or ordinary build time.

The full WASM comparison passes the original four musical scenarios plus a stored frozen-line save case: **38,258,900 channel sample values**, identical PCM, framebuffers and applicable save bytes. The additional saved-piece test exercises six Rust codec calls and compares the 3,836-byte saved bank. Snapshot handoff remains 220 bytes, HTTP callback playback compares another 1,048,576 sample values, the browser API remains 24 exports and linear memory remains 32 MiB without growth. The two original PCM fingerprints remain `baeff971` / `28722449`.

## Footprint

The compact numeric storage is **6,542 bytes**: recipes/overrides 2,832; `XROW` 1,050; `XPAT` 1,080; `PSY_DOWN` 72; tempo/key/mode 570; pluck palette 95; sound palettes 608; pluck rates 19; pad shifts 19; boot patterns 72; mode masks 16; donor IDs 19; packed arcs 90. Immutable names and ABI/slice descriptors are additional. The added mutable state is four 32-bit counters (16 bytes); the existing C voice-name array is retained rather than added.

For the same pinned Rust engine, toggling only `EN_RUST_FACTORY` changes the linked WASM from 2,526,205 to 2,534,175 bytes: **+7,970 file bytes**, **+10,932 code-section bytes**, **+8,640 data-section bytes** and **−11,629 custom-section bytes**. Thus the file delta is not a flash-size estimate or the sum of code/data alone. These numbers include the linked names/ABI machinery and removed C paths. They are recorded with exact binary hashes in [factory-proof.json](rust/factory-proof.json). No MCU linker-map or physical-device timing claim is made.

## Retained C data and ticket boundaries

This inventory prevents a completed factory port from being mistaken for a completed engine port. It describes the R04 branch before integrating the parallel scheduler/effects work.

| Still in C | Follow-up ownership |
| --- | --- |
| Harmonic rule tables `CHT`, `SYN_MAJ`, `CH_SEC`, `CH_BOR`, `AEOLIAN`; chord `OFF`, `SHAPE`, `PERM`, `W7` | [#8](https://github.com/superposition/91/issues/8), harmony and voicing decisions; the ten factory NAIWAN arcs above are already ported |
| Live C scheduling inputs `SECT`, `FILL_OK`, `BL_ROOT`, `BL_RIFF`, `BL_FIG`, `COMP_MASK`, `COMP_GENRE`; local melodic duration/rate rules | [#10](https://github.com/superposition/91/issues/10), musical scheduling; parallel Rust decisions capture these C scalar inputs |
| C0 copies of `FILLS`, `SONG_BARS`, `RATCHET`, `GHOST_SN`, `GHOST_KK`, `CHAIN`, `SLICE` and drum importance/rate rules | Existing `fm1-engine::sequencer` already owns the corresponding live Rust-enabled decisions; retained C declarations/helpers are reference data, not an R04 omission |
| `KITS`, `PAD_SOUND`, `DRONE_SOUND`, `DRONE_FR`, `ACID_SOUND`, `ACID_FM`, `CM_SOUND`, FM/DY programs/routing/gains; legacy style→sound arrays; `EN_TRIM` | [#11](https://github.com/superposition/91/issues/11) / [#12](https://github.com/superposition/91/issues/12), voice implementation. Their display labels and factory selection metadata are ported here |
| `PAD_PAN`, `BREATH`, `START_T`, `VOWEL`, `HAT_INC`, LFO/echo timing and filter coefficients; `DL_LEN`, `HEAD2`, `EN_FXS_DEF`, `PT_MASK`, `RPT_BEATS`, `FLEX`, `DJS` and local performance/mixer coefficient tables | [#13](https://github.com/superposition/91/issues/13), sample/effects/control kernels |
| `MAPS`, `DRUM_GM`, local parameter-value formatting strings (`ONOFF` and small switch-local arrays), mutable user voice labels, screen/page assets | Transport/application/UI tickets; factory map/voice/FX names above are ported, routing/formatting logic is retained |
| `jam_factory.inc`, `song_factory.inc` embedded pack assets | Existing pack codec/planner/application work; unchanged assets, not expanded into a second preset library |

Physical FM-1 validation, MCU ROM/RAM/linker accounting and interactive browser/MIDI testing of this branch remain deferred. No firmware was flashed and no live site/server was changed.

## Reproduce

Use Rust **1.94.1**, a native C compiler, clang/wasm-ld, Node and Python. Build outputs stay under this checkout's ignored `build/` and `target/` directories.

```sh
export RUSTUP_TOOLCHAIN=1.94.1
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown
sh tests/rust_factory_tables.sh /path/to/immutable-r01/source
sh tests/rust_emulator_parity.sh

# Same-Rust footprint comparison; preserve the qualified enabled artifact.
EN_VERSION=0.3.0 EN_RUST_JAM=1 EN_RUST_FACTORY=0 sh web/emu/build_clang.sh
cp build/emu/amb.wasm build/factory/disabled.wasm
node tools/factory_footprint.mjs build/factory/disabled.wasm build/emu-parity/rust.wasm
cp build/emu-parity/rust.wasm build/emu/amb.wasm
```

The immutable-R01 argument is optional for routine CI after the three fixtures' provenance has been established; fixture hashes, selectable C0, Rust tests and the real native C ABI are still checked without it. To maintain the imported compact data deliberately, run `python3 tools/import_factory_tables.py` followed by `cargo fmt --all`; the import checks the pinned corpus hashes first.
