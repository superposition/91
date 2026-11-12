# Shared codecs in the browser and project saves

This extends [R05 / #6](https://github.com/superposition/91/issues/6) and [the original codec proof](RUST_SHARED_CODECS.md). The browser's piece/set/backup, MIDI-map, DX7, updater and debug-frame helpers now execute the maintained `fm1-formats` code. The instrument retains its 24 public WASM exports. A separate, bounded codec module keeps file processing independent of audio rendering.

## Execution and ownership

| Data | Safe Rust interface | Production consumer |
| --- | --- | --- |
| PMGL12/32 | `objects::GlobalParts` | `project_flush_globals`; browser set-list edits |
| SET1, natural padding | `objects::SettingsParts` | `settings_save`; settings-load validation remains shared |
| VOX1, sixteen packed slots | `objects::VoicesParts` | `voices_save`; browser piece/set voice assignment |
| HYP3 header, 376-byte records, progressions | `BankParts`, `bank_header`, `RecordValues`, checked field/import helpers | Actual project save/load; browser bank sharing |
| HYP1/HYP2 | `LegacyKitParts`, `LegacyRecordParts`, `LegacyBankParts` | Typed compatibility encoders; actual legacy project read already uses Rust |
| KND1, PCX1, BSQ1 | `KindsParts`, `ExtensionsParts`, checked extension access/update | Project extension headers; browser migrations and sharing |
| MAP1/2 | checked `MidiEntry`, `MidiMapParts`, borrowed `MidiMap` | Browser learned-control serialization; device restore validation |
| P1PC/P1ST/P1BK | typed constructors/views in `containers` | Actual browser file/code helpers |
| DX7 packed128/canonical155 | `voice`; explicit compatible `voice_file` iterator | Editor import, edit, export, slot transfer; installer file selection |
| OTA/Jam/backup/editor/debug frames | `wire`, `device_wire::dispatch` | Browser updater/device tools and opt-in firmware C primitive bindings |
| CRC32 | shared `crc32` | Browser file checksums and ZIP checksum; firmware Jam transfer checksum |
| Jam packs and sessions | existing validated views/encoders | Existing instrument path; codec WASM exercises both families |
| FWSC logical addressing/product, UFW header/directory/package, JLFS, CRC/ENC/SFC/key/marker/compensation | `package`, `package_write` | Browser firmware reader; actual SDK packager, loader packer, site builder and recovery reader |
| Recovery key cipher, legacy debug35-bit words, logger frames, Jam payload fields | `tool_wire` | Existing Python recovery/debug/logger/Jam tools through the same safe dispatcher |

`EN_RUST_FORMATS` stays independent of the factory flag. C0 implementations remain selectable. No musical parameter, object ID, protocol magic, storage size, checksum scope, or recovery layout changes. Typed byte work does not move transfer ownership, erase scheduling, MIDI interrupt behavior, or browser connection policy.

Compatibility profiles are explicit. Strict MIDI parsers validate checksums and framing. Legacy DX **file** import retains the editor/installer's historical acceptance of unchecksummed Yamaha files and bare 4096-byte banks; the installer retains original packed/single bytes. Updater discovery retains the original permissive identity profile; read requests still validate their payload checksum. Existing unknown/reserved fields are preserved through borrowed views, compatible encoders or template-preserving constructors. Newly saved objects follow the original zero-padding policy.

## Browser boundary

`fm1-codecs-wasm` is a separate no_std cdylib with no imports, allocator, callbacks, shared memory or memory growth. Its memory is fixed at **4 MiB**, including two 1,100,000-byte windows and a 512 KiB stack. The safe dispatcher lives in `api`; the only unsafe code is the narrow exclusive window ABI. Three adjacent 32-byte guards and operation counters provide execution witnesses.

ABI1 uses a 256-byte request header: 32 little-endian scalar words followed by sixteen `(offset,length)` descriptors. Nonempty spans must begin after the header and fit within the supplied request. Outputs are bounded caller storage; errors are typed in the core and stable negative codes across the ABI. The JS runtime copies inputs before entry and outputs before the next call. Its synchronous execution cannot reenter the module.

The build embeds the compiled module into the existing static scripts. It needs no extra network request, MIME configuration, secure origin or browser library. Music and MIDI connection flows remain unchanged. `tools/build_codecs.py` pins Rust1.94.1 and records both source and artifact hashes; the site builder rejects missing/stale codec inputs. The emulator packaging step builds this shared browser dependency; it does not link the sidecar into the audio module.

## Host tools boundary

The existing `fm1-rs` CLI supplies `codec OP` and a bounded `codec-stream` stdio mode. `tools/rust_codecs.py` compiles that host tool with pinned Rust1.94.1, then owns one serialized child pipe per Python process. The child has no network listener, device handle, application state or retained input borrow. Requests are bounded before allocation. Python closes it on exit; failures propagate and a subsequent explicit call may start a fresh child. Windows launches it without a console window. No Python codec fallback masks missing Rust execution.

This keeps command-line logging and MIDI transfers from launching a process for every frame. The allocating host transport is outside the allocator-free core. `fm1pkg_make.py`, `build.py`, `fm1_install.py`, `fm1log.py`, `jam_midi.py`, `fm1_rescue.py`, `fm1_debug.py` and the site package reader delegate their byte operations to this shared implementation. The independent slow CRC in the boot-stub packager remains an intentional cross-check, not the production encoder.

SDK asset selection, boot-stub placement, flash ranges, loader/recovery policy, user confirmation, device discovery and transfer scheduling remain with their existing owners. In particular, USB mass-storage CBW/CSW/CDB transport and guarded development register-command services belong to #19. Music-source compilers remain under #3; their already-qualified Jam/session formats are outside this remaining-codec port. Project publication/flash transaction policy belongs to #7 and browser interaction policy to #23.

## Evidence

* A frozen R01 C exporter uses the original struct definitions and byte writers for every retained object constructor. It also executes actual settings, voices, project, globals, extras and bass save consumers. All 440 immutable source hashes are verified before reproduction. Current C0 and the production C→Rust adapters produce identical bytes.
* **2,923** independently sourced C/JavaScript/Python operation vectors execute in both the safe native dispatcher and actual codec WASM. They cover constructors, signed records, files, package primitives and wire families. Expanded fixtures remain test-only and never enter a production module.
* Native decoder tests vary malformed descriptors, truncation, output capacities and data bytes, checking guard tails. Dedicated constructor tests cover invalid indices, reserved/padding preservation, full voice slots and compatible unknown fields.
* WASM additionally reproduces all **72,448** frozen C DX7 field mutations and all **65,536** Python key blobs. Memory size remains fixed and window guards remain intact.
* Existing player-oriented piece, backup, DX-file, five updater discovery profiles and device API/busy/disconnect/import suites run through the compiled codec client. Source-extracted tests inject that actual client, with no JavaScript codec fallback.
* The real Python SDK packager executes Rust for 1,377 frozen primitive checks and produces a byte-identical **611,251-byte** package using the actual recovery-stub app, loader and three hash-recorded SDK inputs. The independent frozen writer never calls the maintained Rust module. Existing boot-stub integrity tests report **265 checks, zero failures**. Simulated installer/Jam tools and logger self-tests exercise the same host binding without hardware. Host tests cover concurrent callers, oversized/truncated messages, explicit failure/retry and required-executable errors.
* The full seven-scenario C0/Rust emulator suite passes, including saved HYP3/custom-DX playback/save and effect tails. HTTP audio fallback and browser API/resource-lifecycle doubles pass. The generated site preserves all **152** interactive control IDs; all routes, scripts, local assets and nested hosting paths pass source/build checks.

The original browser checkpoint is recorded in [browser-codecs-proof.json](rust/browser-codecs-proof.json); [package-codecs-proof.json](rust/package-codecs-proof.json) records the completing host binding and full-package comparison. Physical device/browser interaction is not claimed by source/Node tests. Root qualifies the combined latest engine base separately. No remaining duplicate browser/host byte-codec implementation is deferred under a generic platform label; the policy boundaries above are separately ticketed.

## Reproduce

```sh
export RUSTUP_TOOLCHAIN=1.94.1
sh tests/rust_browser_codecs.sh /path/to/verified-r01/source
sh tests/rust_device_wire.sh /path/to/verified-r01/source path/to/package.fwsc
sh tests/rust_shared_formats.sh /path/to/verified-r01/source
sh tests/rust_emulator_parity.sh
python3 tools/make_pages_site.py 0.3.0 build/site-codec-check
python3 tests/web_site_test.py build/site-codec-check build/emu
```

The first command compiles the sidecar and native copy bridge, checks fixture hashes and compares frozen/C0/Rust constructor bytes. The immutable-source argument is optional in routine CI; native/WASM goldens, consumer suites, current C0/Rust comparisons and bounds checks still execute.
