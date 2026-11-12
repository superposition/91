# Shared binary codecs: R05 checkpoint

This is the verified implementation slice for [R05 / #6](https://github.com/superposition/91/issues/6), on top of the compact factory port. It adds maintained, allocation-free `fm1-formats` codecs and selects them in real C consumers with `EN_RUST_FORMATS`. It does **not** claim that every remaining storage constructor, device service or browser implementation has moved to Rust. The exact follow-up boundary is listed below.

## Source inventory and current execution

| Format / authority | Rust API | Current consumer |
| --- | --- | --- |
| HYP3 records/banks, `firmware/src/app/project.c` | `storage::Record`, `RecordValues`, `Bank` | `pack_slot` and `unpack_slot` call the Rust copy bridge |
| HYP2 signed records, same source | `storage::LegacyRecord`, `LegacyValues` | `old_bank_load` calls Rust before the original C musical migration policy |
| HYP1 kit, PMGL globals, SET1 settings, VOX1 voices, KND1 / PCX1 extras, BSQ1 bass steps, MAP1/2 maps; `project.c` / `ui.c` | `LegacyKit`, `ObjectView`, `Settings`, `restore_valid`, `midi_map_valid` | Restore, MIDI-map and settings-load validation call Rust; existing record constructors remain C |
| P1PC pieces / P1ST sets, `web/fm1piece.js` | `containers::PieceFile`, `PieceParts`, `SetFile`, `SetParts` | Shared native/WASM-capable library; browser keeps its existing JS helper |
| P1BK backups, `web/fm1backup.js` | `BackupFile`, `BackupObject`, `backup_encode` | Shared library, checked against frozen JS; existing browser file helper retained |
| DX7 VMEM128 / VCED155, `dsp/fm6.inc` | `voice::PackedVoice`, `Voice`, `pack`, `unpack`, `sanitize`, `maximum` | The actual `fm6_pack` / `fm6_unpack` functions select Rust, including loaded-voice playback |
| Yamaha bulk/single and custom slot/clear SysEx, `usb.c` / `main_fm1.c` | `voice::DxMessage`, `IncomingKind`, `classify_legacy`, bounded encoders | `usb.c::dx_end` selects the Rust structural classifier; collection/publication and service policy stay C |
| Backup/editor frames, `backup_proto.h` / `dx_proto.h` | `wire` nibble/checksum, frame/chunk, dump/voice, ack/about codecs and typed views | C frame builders, chunk parser and shared checksum select Rust |
| Jam envelopes, `app/jam_usb.h`, browser capability query | `wire::JamRequest`, `JamReply`, `jam_reply` | Bounded library views; current transfer operation/state service retained |
| OTA framing, `ota.c` / `web/fm1ota.js` | `wire::pack7`, `unpack7`, `OtaMessage`, `encode_ota`, `decode_ota` | Differential library coverage; C update service still uses its existing primitives |
| Package primitives, `tools/fm1pkg_make.py` / `web/fm1pkg.js` | `package` CRC16, ENC/SFC, chip key, FWSC logical addressing, UFW directory, JLFS entry, body marker and compensation | Differential library coverage; existing SDK package assembly and recovery path retained |
| Jam v1/v2 packs and 20-byte session | Existing `jam` / `session` modules | Existing Rust validation/planning/session integration, unchanged by this slice |

`EN_RUST_FORMATS=1` is an independent emulator build flag: `EN_RUST_FACTORY=0` does not disable it. `EN_RUST_FORMATS=0` retains the C codec path with the other Rust slices enabled; `EN_RUST_JAM=0` retains the all-C reference. Hardware defaults remain C. No new public WASM export is added.

## Byte layouts and compatibility

The codecs read/write explicit integers, never a Rust struct image or `repr(packed)`. `fm1-formats` is `no_std`, forbids unsafe code and has no allocator. File views borrow existing bytes; encoders use caller-provided storage. `CodecError` distinguishes length, magic, version, index, range, checksum, nibble, capacity and state failures.

The HYP3 record remains **376 bytes**:

| Offset | Bytes | Meaning |
| --- | ---: | --- |
| 0 | 2 | Scale pool, little endian |
| 2 | 72 | Six patterns × six 16-bit lane masks |
| 74 | 176 | Setting deltas from each parameter minimum; `FF` means recipe |
| 250 | 6 | First half of the packed pluck line |
| 256 | 18 | Packed acid line |
| 274 | 6 | Second half of the pluck line |
| 280 | 96 | Six lanes × sixteen modifier bytes |

The bank is a 16-byte header plus ten records (**3,776 bytes**), optionally followed by ten six-byte progressions (**3,836 bytes** total). Missing parameters and `FF` settings keep the caller's recipe. The legacy chance-in-sync value becomes zero as C did. `decode_legacy` preserves the original clamped output and validity result even for bad records; the checked `decode` rejects bad scales/parameters. New-save encoding zeros spare bytes and clamps deltas exactly like `pack_slot`; a raw existing-record copy preserves all bytes.

HYP2 uses a 20-byte header and ten records of `2*npar + 74` bytes, with signed little-endian parameters. The test covers counts 1/76/98/154 and all four old revision states; 154 is the largest full ten-piece record count fitting the original 3,840-byte store. The decoded rate/tone compatibility fields are handled in Rust. C still applies the existing style→sound, acid-answer and old DX slot-number policies afterwards. The independent oracle captures both the actual C pre-policy parameters and final application result.

The shared file envelopes are unchanged:

```text
P1PC: header14 | record376 | progression6 | [voice128] | metadata[ml] | CRC32-LE
P1ST: header27 | piece387[np] | voice128[nv] | metadata[ml] | CRC32-LE
P1BK: header8  | metadata[ml] | { id8, length16-LE, payload }[count] | CRC32-LE
```

JSON is opaque metadata: unknown keys, text encoding and property order survive a binary round trip. Existing unknown flag/reserved bytes remain in borrowed views. P1BK unknown object IDs and checksum-covered trailing bytes are retained; restoration policy is separate. Current-index parsing checks genre/piece/voice references. Explicit `parse_compatible` views preserve structurally valid older/future index values without using them as unchecked indices. The application must use the record decoder for musical setting/scale validation; a checksummed file envelope alone does not establish musical validity.

DX7 conversion intentionally canonicalizes: packed fields are masked, parameter maxima are enforced, and name bytes outside ASCII 32..126 become spaces. Raw borrowed voice views preserve the input bytes. The C ABI retains its existing **156-byte output requirement** (155 data bytes plus NUL); Rust runtime voice values remain exactly 155 bytes. This also supplies the shared sanitization API for the separate FM6 engine port.

Backup and Jam raw bytes use high-nibble then low-nibble encoding. Backup checksum scope begins at the object byte; Jam scope begins at the operation byte. Yamaha/custom voice checksums cover voice data. OTA uses an **LSB-first 8→7 bitstream** and an eight-bit one's-complement payload checksum, not the seven-bit backup checksum. These operations have separate named APIs.

Package reads preserve raw FWSC bytes and explicitly skip the twenty identity markers when addressing the logical image. UFW headers/directories, JLFS headers, CRC16-0x1021, ENC/SFC, chip-key blobs and CRBM markers match the original Python implementation. Compensation is a pure four-byte calculation; it neither modifies a body nor writes flash. The pinned stub, SDK assets, loader and recovery scripts are untouched.

## Safety and execution evidence

The bridge keeps C-owned state and bounded fixed-copy calls. It checks null pointers, lengths, indices and output capacities; non-null allocation validity/disjointness remains the documented caller obligation. Parameter/word copies support unaligned storage. Voice conversion supports overlap by copying its source before writing. No pointer is retained. Four 32-bit counters (16 mutable bytes) at diagnostic indices 36–39 report decodes, encodes, rejections and copied bytes through the existing diagnostic export.

The immutable R01 source is verified against all 440 file hashes before C/JS/Python exports are reproduced. Hash-pinned fixtures cover:

- All 283 DX7 input byte positions × 256 values, plus 256 complete input/output vectors.
- All 376 packed-record byte positions × 256 values (**96,256 mutations**), 32 complete encodes and 128 HYP2 revision/count/error cases.
- Twelve stored-object schemas, every byte mutation and truncation through two bytes beyond each object.
- Original USB structural classification, all relevant lengths and seven-bit header mutations, with actual extracted C `dx_end` source.
- Five piece files, three sets, four backups, pack7/chunk/OTA vectors from the original JS modules, including unknown flags/trailing bytes.
- CRC16/ENC/SFC/key/marker vectors, all 65,536 chip-key blobs, UFW/JLFS and compensation outputs from the original Python builder.
- Bounded decoder mutation loops, explicit bad index/version/checksum/nibble/length cases, guard bytes, rejected encodes and ABI overlap/unaligned cases.

`tests/rust_shared_formats.sh` compares both the current C0 export and the actual C→Rust API export byte-for-byte against the frozen C JSON. It also runs the existing `dxproto_test.c` with Rust helpers selected. The C adapter intercepts the style-policy call only to capture its input; the actual frozen C policy still runs. USB classifier extraction changes only the single-thread publication barrier in the test harness, not the classifier's source.

The actual WASM saved-piece test loads an HYP3 bank and VOX1 user voice, plays the legacy page, saves and compares PCM, screen and the 3,836-byte saved bank against C0. It witnesses Rust format decode/encode calls, rather than checking only duplicate library functions. The original factory/saved-v1 fingerprints, v2 session lifecycle, legacy flow, snapshot handoff and HTTP audio fallback remain exact. Full counters, binary/fixture hashes, test totals and linked footprint measurements are in [shared-codecs-proof.json](rust/shared-codecs-proof.json).

With only `EN_RUST_FORMATS` toggled, this checkout's linked WASM changes from 2,534,254 to 2,531,838 bytes: file **−2,416**, code section **+4,719**, data section **+544**, custom sections **−7,711**. File shrinkage includes metadata changes and is not an MCU flash or timing claim. These measurements cover the codecs actually linked into the emulator; unused package/tool APIs are not charged as live emulator code. Linear memory remains 32 MiB without growth.

## Explicit remaining acceptance for #6

1. **New storage-object constructors:** PMGL, SET1, VOX1, PCX1/KND1/BSQ1 and MIDI-map new-save assembly still use existing C/JS constructors. Rust provides validated field views and lossless copies of existing objects; this checkpoint does not replace every constructor with a typed Rust encoder.
2. **Browser distribution:** connected tools still invoke their existing JS file/packet helpers. Frozen JS differential tests guard compatibility, but the browser is not yet loading a shared Rust codec module. The emulator retains its 24-export interface.
3. **Remaining device-service bindings:** the C OTA service still calls its original CRC/ENC/pack7 primitives, despite qualified Rust equivalents. DX service checksum/dispatch, Jam transfer operation policy and per-command debug/telemetry schemas remain in C. USB collection/IRQ/publication are deliberately unchanged and belong to hardware/driver qualification.
4. **Platform qualification:** this branch has native C/Rust and clang-WASM evidence. Physical MIDI/device writes, MCU linker-map/ROM/RAM/timing and Emscripten browser/device testing are not claimed. Root must rerun qualification after combining parallel DSP slices.

These are concrete follow-ups; #6 must not be represented as fully closed by this checkpoint alone.

## Reproduce

```sh
export RUSTUP_TOOLCHAIN=1.94.1
cargo fmt --all --check
sh tests/rust_shared_formats.sh /path/to/immutable-r01/source
cargo test --workspace --release --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
sh tests/rust_emulator_parity.sh

# Isolate the linked codec delta; keep the qualified enabled artifact.
EN_VERSION=0.3.0 EN_RUST_JAM=1 EN_RUST_FACTORY=1 EN_RUST_FORMATS=0 sh web/emu/build_clang.sh
cp build/emu/amb.wasm build/formats/disabled.wasm
node tools/factory_footprint.mjs build/formats/disabled.wasm build/emu-parity/rust.wasm EN_RUST_FORMATS
cp build/emu-parity/rust.wasm build/emu/amb.wasm
```

The immutable-source argument is optional in routine CI: fixture hashes, native C0/Rust differential exports, codec tests, no_std check and protocol guard tests still run without it. The source-generation adapters are test-only and never run in production builds.
