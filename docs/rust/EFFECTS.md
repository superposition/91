# R12 effects and master-bus checkpoint

This checkpoint enables allocator-free Rust effect/control kernels in the real
emulator render path. It preserves the C fallback and the existing delay storage.
It does **not** finish the whole mixer or qualify PI32 firmware. Issue #13 stays
open for the remaining C arithmetic and physical link/memory/timing acceptance.

## Signal flow and the remaining C boundary

```mermaid
flowchart LR
  V[C voices and per-voice sums] --> M[Rust mixer gains and differential gain]
  M --> D[Rust sidechain, gate and noise merge]
  D --> B[Rust track-bus filter and phaser]
  B --> E[C send sum, Rust tape and Echoplex]
  E --> P[C predelay push and dry scaling]
  P --> T[Rust reverb tank on the existing staged job]
  T --> R[C tank-return addition]
  R --> F[Rust performance ring and tone effects]
  F --> L[Rust compressor and limiter]
  L --> O[Rust master gain, clip and output conversion]
  O --> H[24-bit output]
  O --> U[Fixed-level USB output]
```

The migrated kernels include tape/Echoplex, plate/shimmer/gated reverb, chorus,
LP/HP/sweep/Talk/Wah, tape colour, flanger, phaser, ring modulation, crush, vinyl,
repeat/reverse/stop/scatter/varispeed/scratch/grains/reverse swell/Flex playback,
mute/solo/intro/gate/pump gains, mask priority, fades, settings/coefficient updates
and performance engagement transitions. The audio path calls these kernels; the
new APIs are not unused alternate implementations.

The following **mixer arithmetic still runs in C** in `amb.c`:

- The voice renderers' intrinsic additive pan/send operations, `mx_begin`'s
  before-buffer copies, and the pad/FM accumulation in `render_chunk`.
- Routing the plain pluck and differential drum contribution into the track bus.
- The bass/acid ducked gain and send accumulation after their staged work finishes.
- Combining the echo input from sends, throw and the track bus; reverb/predelay
  input construction and `push` quantization; dry-mix scaling and duck scaling.
- Adding the tank return before `fx_master` invokes the Rust performance/tone
  kernels. Delay-buffer allocation/initialization, render scheduling, staged job
  submission/join/fencing, and the original reset orchestration also remain C.

`mx_gains` and `mx_end`'s differential gain, sidechain/gate/noise processing,
differential drum crush, the track-bus filter/phaser, dynamics and final output
conversion run in Rust. Voice synthesis belongs to the separate voice tickets;
this checkpoint makes no claim that those ports or the remaining mixer sums are
complete. Grid callback orchestration (`flx_tick`, refresh/leave checks) remains
at the scheduler boundary; its effect-engagement callbacks enter Rust.

## State and bounded storage

`fm1-engine` remains `no_std` with `#![forbid(unsafe_code)]`. Fixed array borrows
express delay lengths. Scalar packets use `repr(C)` and initialized numeric
fields; there are no foreign bools, Rust enums or retained pointers in the ABI.
`tools/effects_abi.py --check` validates maintained scalar adapters and all FX
symbol IDs. Generated control/engagement layouts assert each field offset, size
and alignment in both languages. Other packets have matching size assertions.

The bridge's unsafe operations create scoped, aligned, initialized, disjoint
borrows. The caller must uniquely hold mutable buffers throughout each call.
There are no foreign callbacks during a borrow. Delay payloads are neither
allocated by Rust nor copied into Rust stack arrays.

| Existing delay payload | Shape | Bytes |
| --- | --- | ---: |
| Performance ring | 21,000 stereo frames, i16 | 84,000 |
| Tape | 21,800 mono i16 samples | 43,600 |
| Echoplex | 7,100 mono i16 samples | 14,200 |
| Reverb including predelay | 34,962 i16 words | 69,924 |
| Shimmer | 2,048 f32 samples | 8,192 |
| Chorus | 2 × 512 f32 samples | 4,096 |
| Tape colour and flanger | 2 × 2 × 512 f32 samples | 8,192 |
| Limiter history/gains/minima | 3 × 64 + 8 f32 | 800 |
| **These delay payloads combined** | | **233,004** |

This table is not a complete firmware/pool map: voice buffers, staged output
buffers, state, UI/storage and linker alignment are additional. The existing C
payload declarations and placement attributes remain unchanged. The four new
diagnostic `AtomicU32`s use 16 bytes; Rust adds no mutable delay payload allocation.

The reverb bridge borrows exactly the **33,383-word tank after the 1,579-word
predelay**, not the whole shared allocation. The worker's predelay read stays a
C scalar load. Its disjoint foreground writer therefore does not alias the Rust
tank borrow. Chorus receives its private delay and temporary pad output, not a
whole mutable borrow of the concurrently consumed pad ring. The performance ring
requires serialization with spectrum/UI reads; serialized WASM is the qualified
application backend. Native threaded output parity is additional evidence, not a
claim of complete C race analysis or physical worker qualification.

## Transitional cost and preserved details

The C adapters still pack/copy scalar state around foreign calls. Source-level
packet sizes are below; these are **not measured stack high-water marks** or
compiler memory-traffic counts.

| Call frequency | State bytes | Config bytes |
| --- | ---: | ---: |
| Master dynamics, per block | 72 | scalar settings separately |
| Echo, per sample | 56 | 48 |
| Tank, per sample | 96 | 24 |
| Tone, per sample | 232 | 240 |
| Performance ring, per sample | 136 | 116 |
| Chorus, per block | 16 | scalar amount |
| Mask/gain/coefficient control, per control interval | 708 | 288 |
| Engagement, per transition | 216 | 288 |
| Track bus, per block | 56 | 40 |
| Sidechain, per block | 8 | 36 |
| Vinyl, per block | 124 | 40 |

State is copied back as well as captured. Per-sample FFI and scalar marshalling
are an intentional transitional cost. PI32 instruction cycles, interrupt stack,
link/map reserve and firmware size remain unqualified; browser/native performance
does not establish an MCU deadline. No target flags were enabled or hardware
flashed. The emulator still declares a 1 MiB WASM stack and 32 MiB initial memory.

The port retains binary32 expression order and legacy fast-math approximations;
it does not use FMA, libm or allocation in the musical core. C builds explicitly
use `-ffp-contract=off`. It preserves the shared `tp_ph` used by tape wow and the
track-bus phaser, the quarter-rate tape, half-rate performance/vinyl history,
rounding, ring positions, and reset behavior. The limiter's ring has 64 slots;
the frozen implementation's first impulse appears at index **63**, and that
observed latency is deliberately preserved. Limiter reset keeps its checksum
phase and coefficients. The fixed USB tap stays independent of master gain and
uses the full tanh even when shed level 3 selects the cheaper main-output clip.
Existing staged work placement and load-shedding locations are unchanged.

`web_rust_stat(28..31)` reports master, echo, tank and combined colour sample-call
counts. The last is an aggregate of chorus + performance ring + tone calls, not
a unique-frame or audio-quality metric.

## Evidence and reproduction

Use Rust **1.94.1**, Node **22.14.0**, the existing pinned WASM clang wrapper and
the immutable R01 baseline. `effects-proof.json` records exact artifacts/results.

```sh
export RUSTUP_TOOLCHAIN=1.94.1
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p fm1-types -p fm1-formats -p fm1-engine -p fm1-rt --target wasm32-unknown-unknown --locked
cargo build --release --locked -p fm1-jam-bridge --no-default-features --target wasm32-unknown-unknown
python3 tools/effects_abi.py --check
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source extra-genres
sh tests/rust_effects_workers.sh ../baseline-2026-11-11/source
sh tests/rust_emulator_parity.sh
node tests/host/emu_effects_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
```

Native tests verify all 440 baseline source hashes, compile frozen C, fresh C0
and Rust builds, then compare PCM, USB, scalar state, periodic complete ring
payloads and shared RNG. Twenty-three cases cover all 19 genres, max feedback,
mask/FX transitions, direct external-clock engagements, mute/solo/gate/ramp
changes, Stop/restart, freeze, release tails, shed level 3, and independent direct
impulse/null chains for all three tank types: **12,246,104 frames**. The direct
impulse hash is `03e7f876`; null is `2cd4e83a` with exactly zero peak. Seven new
Rust tests check impulse latency/reset, nonfinite rejection, fixed USB gain,
maximum-feedback tails, mask priority/ramps and grain RNG draw count. The full
workspace contains 38 passing tests at this checkpoint.

The native worker harness passes 24 frozen-C/Rust combinations: serial, deferred,
pthread and busy/refused submissions; three tank types; and both retained work
placements. Synthetic abandoned-live-thread cases are intentionally excluded;
they do not establish safe physical retirement.

The full WASM suite compares **40,058,180** Float32 PCM values and an additional
1,048,576 in the HTTP fallback, with unchanged 24 exports, no playback memory
growth, and 32 MiB memory. The new real-panel FX bank/hold/release/tail scenario
compares 2,769,480 values, is audible (peak 0.453861), and has fingerprint
`fd659ab9` against both fresh C0 and the independent frozen R01 WASM. Existing
factory/saved-v1 fingerprints stay `baeff971` / `28722449`; save/restart and
framebuffer parity also pass. These are Node/WASM qualifications, not new manual
desktop/mobile/audio-device/MIDI QA claims.

The complete current Rust-enabled WASM is 2,456,251 bytes versus 1,940,077 bytes
for fresh C0. That comparison includes **all active Rust migration flags**, not
an isolated measurement of this ticket's code size.

The remaining acceptance is concrete: port the mapped C mixer arithmetic, then
qualify the merged physical build's pool reserve, absence of soft-double helpers,
target link/map, interrupt stack, timing, worker exclusion/retirement and device
output. The emulator/native checkpoint does not close those items.
