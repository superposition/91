# R10 block configuration arithmetic

This follows [VOICES_SETUP.md](VOICES_SETUP.md) after merging canonical pad, FM6, formats and mixer work at `72a4c6658f3c81d531761cd7a613a553c4f94926`. The live drone, acid, pad, pad-bus, drum and pluck adapters now capture existing values and ask [voices/config.rs](../../rust/fm1-engine/src/voices/config.rs) to calculate their fixed block configurations. Gain, modulation routing, resonance/drive normalization, glide selection, bend, sound fallback, stab selection and envelope-time conversion retain the original binary32 association.

The bridge borrows the existing 176-value parameter array; it does not copy that array. Scalar captures are 100 bytes for drone, 60 for acid, 104 for pad, 32 for pad bus, 84 for drums and 28 for pluck. Each is built once per block. Existing output configuration sizes and all voice capacities remain unchanged. There is one additional bounded configuration FFI call per block, no per-sample packet, no allocator, no persistent cache and no new state/diagnostic storage. Read-only recipe references point to the same exported immutable arrays already used by C; the portable builders do not instantiate a second production table bank.

Each call borrows parameters/raw inputs immutably and writes only its separate configuration output. All references end before the sample kernel takes its mutable voice/buffer borrows. No builder invokes C or retains a reference, pointer or callback. C/Rust size assertions cover every raw capture, and the C entry point asserts the parameter count. The existing worker protocol must keep the captured/read parameter values stable until completion; ownership/quarantine changes remain #14/#19 work.

The C capture files are `rust_tonal.inc`, `rust_pad.inc` and `rust_voice_config.inc`. Their C0 calculation bodies remain selectable for the independent adapters; normal hardware C0 rendering remains unchanged. The production Rust paths only gather values, call a Rust builder and publish the existing ABI value. Builder executions use the previously reserved statistic 47, with no additional allocation.

## Evidence

[voices-config-proof.json](voices-config-proof.json) records source hashes, exact input/output artifacts and test results. The new independent matrix contains 1,536 configurations, 256 per builder, and compares every raw capture, parameter and output word between frozen R01, current C0 and Rust. It exercises all sound recipes, invalid sound fallback, all LFO destinations, all squash targets, manual/held/line/slide settings, filters, changing genres, gains, bends and modulation values. The portable test enforces the recipe/destination/target coverage.

All previous 670 isolated voice state/PCM cases remain exact. The combined native build enables real Rust FM6 and formats and preserves all 43 frozen full-instrument traces. Seven current WASM scenarios compare 45,780,596 Float32 values, including four loaded-user FM6 algorithms; the original R01 F4 case and HTTP fallback also pass. F4 observes 29,145 configuration calls, one for every applicable sample-kernel block. The public WASM interface remains 24 exports with fixed 32 MiB memory. USB simulation and frozen-source worker placement checks remain part of qualification. Rust 1.94.1 formatting, 85 workspace tests and clippy with warnings denied pass. Refreshed host release costs retain every trial; physical deadlines and stack/RAM qualification remain separate.

```sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_voice_config.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
```

R10 is still open for the remaining numeric per-note and parameter-application initialization: `cm_trigger` duration/velocity, acid pending decay and voice-related `apply`/`en_init` arithmetic. A final bounded follow-up will move those into Rust. C hardware/application/MIDI/FM dispatch and publication remain explicit adapter policy, while harmonic construction is assigned to #8 and application orchestration to #23. FM6 internals belong to #12; effects/mixer to #13; worker and buffer publication to #14/#19. This checkpoint performed no hardware I/O, live-server change, push or remote-ticket action.
