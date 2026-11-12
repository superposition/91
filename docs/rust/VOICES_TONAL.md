# Drone and acid DSP checkpoint

This continues [R10 / #11](https://github.com/superposition/91/issues/11) after the [percussion/plain-pluck checkpoint](VOICES.md). The Rust-enabled emulator now executes the complete drone/bass and acid sample loops, including their control updates every 32 samples. Pad DSP remains the next implementation work. C0 and the default physical build retain the original C functions.

[`voices/drone.rs`](../../rust/fm1-engine/src/voices/drone.rs) and [`voices/acid.rs`](../../rust/fm1-engine/src/voices/acid.rs) borrow the existing C voice object, control counter, sine table and mono output buffer for a block. They contain oscillator selection, phase updates, polyBLEP, lookup FM and phase distortion, TPT filter state/coefficients, drive, glide, gate/envelope evolution, ramps, level publication and denormal flushing. No C callback occurs while these references exist. The portable crate still forbids unsafe code and requires no allocator.

| Value | Bytes | Storage |
| --- | ---: | --- |
| Drone/bass voice | 96 | Existing oscillator/filter/envelope object, same layout |
| Bass-line envelope | 12 | Two existing floats and one byte grouped into an aligned ABI object |
| Drone block configuration | 124 | One bounded scalar packet per block |
| Acid voice | 108 | Existing oscillator/filter/envelope object, same layout |
| Acid block configuration | 88 | One bounded scalar packet per block |

The line object includes three padding bytes, with no duplicated envelope state. Voice capacities, the 1,025-value borrowed sine table, buffers, feature flag and eight reserved diagnostics remain unchanged. Counters 43 and 44 record the drone and acid blocks. The C adapter supplies immutable parameter/sound values and the original scalar expression prefixes; Rust preserves the subsequent operation order. The drone cutoff's envelope and modulation terms remain separate inputs so their addition order is retained. Pitch modulation and keyboard bend also remain separate additions.

The 32-sample counters continue across irregular blocks and explicitly wrap as unsigned integers. Acid's block-local sync ratio, PWM width, laser factor and drive defaults still reset at each block before the next control boundary, matching the original behavior. The drone's filter and bass-line envelope retain their separate control/sample cadences. Six-operator FM bass FIFO playback is left to #12.

## Evidence

[voices-tonal-proof.json](voices-tonal-proof.json) records hashes, vectors, peaks and host timing trials.

- The untouched R01 kernels generate 222 cases covering all 24 drone sounds and 31 acid sounds, four active/held/line/released variants per sound, plus both silent early exits. Inputs include glide and bend, filter types, resonance/drive extremes, accent/wow, phase wrapping and control counters starting at 0, 7, 31 and `0xfffffff0`. Each case runs 256 calls with 0, 1, 31, 32, 33, 127, 128, 129 and 256 frames.
- Every mono output value and private voice/envelope/control field matches byte for byte in the native executable comparison. Portable Rust kernels reproduce the independent frozen fingerprints. C0 and Rust also export identical complete input/configuration/fingerprint fixtures. The vectors contain 4,595,400 Float32 outputs; all are finite, with maximum observed magnitude 1.44339418 under the asserted bound of 64. The resulting waveform difference is exactly zero for every compared sample.
- The full 43-case immutable-source native suite again matches 161,688,822 integer PCM values, MIDI and private scheduling state; the motif/gate coverage audit passes. All six current WASM cases match 41,028,380 Float32 values, UI and saves. HTTP adds 1,048,576 values. The F4 case also matches the original R01 WASM, preserves 24 public exports/fixed 32 MiB memory and observes 4,957 drone and 6,047 acid Rust blocks.
- Existing C0/Rust/R01 handoff checks retain `eff97ff8`. Serial, deferred, pthread and busy-worker regression output also matches the frozen C source, covering the second-stage drone/acid worker ownership boundary.
- Rust 1.94.1 formatting, 52 workspace tests and clippy with warnings denied pass on this isolated branch.

The release host measurements use 128-frame calls with periodic retrigger. Measured Rust/C median cost ratios range from 1.24 to 1.54; measured Rust costs range from about 12 to 50 ns/frame. The proof retains every trial. These host measurements include adapters and retrigger work and establish no physical-target speed or deadline claim.

```sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_tonal_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_handoff.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_effects_workers.sh /absolute/R01/source
```

## Remaining R10 work

Pad sample/control DSP remains C. Drone's slower chunk-rate envelope update, voice triggering/release/parameter setup and kit/sound literals also remain C. The new scalar adapter [`rust_tonal.inc`](../../firmware/src/dsp/rust_tonal.inc) explicitly retains parameter-to-block setup; it does not duplicate a per-sample voice bank. The active noise generator remains the safe Rust vinyl implementation from #13. Application dispatch, MIDI emission and FM6/allocator paths remain their separate boundaries. Hardware compiler/backend, atomic, stack/RAM and deadline qualification are deferred; no device or live server was touched. This checkpoint does not close all of #11.
