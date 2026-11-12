# Voice DSP checkpoints

[R10 / #11](https://github.com/superposition/91/issues/11) separates the analog voice sample loops from the surrounding application. This first checkpoint moves all six percussion lanes and both plain-pluck voices into safe `no_std` Rust. The active noise/vinyl generator already runs in the separately tested Rust effects module. Pad, drone/bass and acid DSP remain the next work; this checkpoint does not close the whole ticket.

[`voices.rs`](../../rust/fm1-engine/src/voices.rs) operates on fixed state and borrowed buffers. Under `EN_RUST_VOICES`, the existing C drum/pluck objects have the matching Rust ABI layout. A bridge call borrows the six drums or two plucks, the read-only 1,025-value sine table and four disjoint mix/send buffers for the duration of a block. The shared drum noise seed is borrowed exclusively for the percussion call. No callback occurs during those borrows, and no pointer survives them. The original C0 definitions and functions remain selectable; the hardware build still selects C0.

| State or input | Size | Capacity/lifetime |
| --- | ---: | --- |
| Drum oscillator/filter/envelope/pending state | 104 bytes | Six existing lanes, 624 bytes total |
| Plain-pluck phase/envelope/age state | 28 bytes | Two existing voices, 56 bytes total |
| Percussion configuration | 108 bytes | One initialized scalar packet per block |
| Pluck configuration | 44 bytes | One initialized scalar packet per block |
| Sine lookup | 4,100 bytes | Existing C table borrowed read-only; no duplicate |

The bridge updates existing storage directly; there is no parallel runtime bank, allocator, state copy or configuration packet per sample. Eight reserved execution counters occupy 32 bytes in the Rust-enabled emulator; counters 40 and 41 currently record percussion and pluck blocks. The public WASM interface retains 24 exports and fixed 32 MiB memory.

Percussion preserves kick sweep/click/rumble/drive, snare noise/body, clap bursts, both metal/noise hats, FM percussion, retrigger fades, track ramps, send accumulation, denormal flushing and lane release thresholds. Its unsigned LCG explicitly wraps and consumes words in the original lane order. The hats retain the original multiplication association: noise amount times the signed raw word, then normalization. Plucks preserve FM lookup/interpolation, negative phase wrapping, onset ramp, age wrap, decay, alternate voice panning, sends, and the distinct muted/DX cleanup behavior. C still initializes/triggers the voices, supplies sound/kit values and emits scheduled MIDI note-offs.

## Verification

[voices-percussion-proof.json](voices-percussion-proof.json) records exact fixtures, artifact hashes and timings.

- The immutable R01 kernels supply 136 input/state/PCM cases: fourteen kits across each of six isolated lanes and all lanes together, twelve plain-pluck sounds at three notes/decays, plus muted and DX cleanup. Every case uses 256 calls with sizes 0, 1, 31, 32, 33, 127, 128, 129 and 256, preserving state across calls. Raw output and private voice state compare byte for byte, including RNG, pending fields and pluck age wrap. Portable tests reproduce the frozen fingerprints using the real Rust kernels; C0 and Rust executable fixture exports match the same source fixture.
- These isolated comparisons cover 11,260,800 Float32 output values. Outputs are finite and remain within the asserted bound of 8; actual peaks are recorded per case.
- All forty-three full frozen-C native traces still match: 161,688,822 integer PCM values, 209,246 MIDI messages and complete scheduling state. The existing motif/gate execution audit passes.
- The six current full WASM cases match 41,028,380 Float32 values, framebuffers/LEDs, stored pattern and effects paths, and save/restart. HTTP adds 1,048,576 values. The F4 case also matches the original R01 WASM and observes 6,047 Rust percussion blocks and 6,047 pluck blocks. C0/Rust/R01 snapshot/staging/USB-response checks retain `eff97ff8`.
- Pinned Rust 1.94.1 formatting, 51 workspace tests and clippy with warnings denied pass on this isolated branch.

Release host measurements include 128-frame block adapters, clearing four mix buffers and periodic retrigger. Median Rust/C ratios range from 0.92 to 2.04 across the measured cases: this is a behavior-preserving port, with a measured host cost increase for several voices. The mixed percussion cases take about 142–145 ns/frame on this WSL host; the measured pluck cases take about 19–63 ns/frame. These are host observations, not physical-target timing or deadline qualification. The proof retains every trial rather than claiming a speed improvement.

```sh
cargo +1.94.1 fmt --all --check
cargo +1.94.1 test --workspace --locked
cargo +1.94.1 clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_voices_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_handoff.sh /absolute/R01/source
```

## Remaining implementation

The subsequent [drone/acid checkpoint](VOICES_TONAL.md) moves those sample loops and their 32-sample controls into Rust. Pad sample/control DSP, slower drone-envelope updates, voice triggering and parameter-to-coefficient setup remain C, together with kit/sound literals and application/MIDI glue. The active Rust vinyl generator is reused from #13, with its existing paired noise seeds and half-rate hold behavior. Six-operator FM and allocation remain #12; mixer arithmetic belongs to the concurrent #13 follow-up. No physical firmware was read, written or flashed, and no live server changed.
