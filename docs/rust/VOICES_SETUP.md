# R10 analog voice controls, note lifecycle and immutable recipes

This checkpoint follows the percussion/pluck, drone/acid and pad kernel ports for [#11](https://github.com/superposition/91/issues/11). `EN_RUST_VOICES=1` now also executes slow random motion, tempo LFOs, the drone's chunk envelope, drum/pluck/acid attacks, pad handoff/shared-note transitions and stabs, bass target/line application, manual key allocation/legato/release, and panic cleanup in safe Rust. Hardware remains on the selectable C0 path.

The portable code is [setup.rs](../../rust/fm1-engine/src/voices/setup.rs). Ten motion states and both LFO states directly reuse their existing C layout (16 bytes each). All existing voice counts and state sizes remain fixed. The common xorshift stream keeps its draw order, including motion expiry and zero-sized direct helper calls. The actual `en_render` path never calls the full chunk-control chain for zero frames.

Sixteen immutable numeric tables now live in [data.rs](../../rust/fm1-engine/src/voices/data.rs): fourteen kits; 24 pad/drone and 31 acid recipes; drone FM ratios, acid FM parameters and twelve plain pluck recipes; pad pans, breath factors, initial breath phases and vowels; hat increments and GM notes; LFO rates; genre/piece trims; and the common 1,025-value sine table. The bridge exports typed read-only static views, including explicit initialized ABI padding. C uses these views directly. There is no copied runtime table cache, extra voice bank, allocator or telemetry allocation. C0 retains its original private definitions behind the flag.

## Call and borrow boundary

```mermaid
flowchart LR
  A[C command or chunk control] --> B[Capture validated scalars and existing state pointers]
  B --> C[Scoped Rust mutation of fixed voices or controls]
  C --> D[Return bounded values and MIDI events]
  D --> E[C publishes UI and dispatches MIDI or FM]
```

The setup bridge borrows each mutable state region exclusively for one serialized foreground call. Immutable config and recipe rows cannot alias the mutable voices. Motion/LFO calls copy only their single 32-bit RNG word across the boundary. No pointer, reference or callback survives a call. C dispatches note events only after the Rust borrows end; these setup calls do not invoke C callbacks while holding voice references.

The event result holds at most eight messages (100 bytes including length), enough for four pad old-note-offs followed by their new-note-ons. Individual bass/acid/manual attacks need at most two. A manual key call captures 44 bytes and returns 104 bytes, including the bounded events and held/glow publication. Drum attack input is 20 bytes; acid attack input is 36 bytes. Four immutable kit-row references are borrowed without copying the kits. ABI sizes are asserted in C and Rust.

Allocation preserves the original ordered searches: quietest algorithm voice, quietest releasing voice, then oldest held voice using the original `u32`-to-`f32` comparison. Equal candidates retain the first index. The C0 allocator's valid-state assumption that an oldest candidate compares below `4e9f` is explicit; Rust does not promise compatibility with the original undefined out-of-range allocator state. Held-key lists keep eight entries per voice, remove a duplicate before deciding legato, evict the oldest at capacity, and restore the last still-held key on release. The pad keeps four voices and the pluck keeps two.

The separate pad-render MIDI callback is unchanged: it writes only the disjoint original C queue and cannot access voice/runtime/config/output references or reenter rendering. Main breathing producers also use that queue from the serialized parameter/init paths. As documented in [VOICES_PAD.md](VOICES_PAD.md), cross-core producer/consumer synchronization remains assigned to [#14](https://github.com/superposition/91/issues/14) and [#19](https://github.com/superposition/91/issues/19). Serialized state/MIDI parity does not qualify the asynchronous hardware handoff.

## Evidence

[voices-setup-proof.json](voices-setup-proof.json) records the independent artifacts and full trace hashes. `tests/rust_voice_setup.sh` compiles the real selected engine three ways: immutable R01, current C0 and current Rust. It checks every read-only table bit and all 606 envelope-knob values, then compares 94 scenarios with 1,024 calls each. Coverage includes all twelve LFO rates and seven shape/default branches, phase wrap and RNG draws, motion expiry, drone attack/decay extremes, held-list overflow, duplicate removal, pad stealing/release, handoff/shared-note/stab transitions, bass line/target changes, acid attacks/slides and panic cleanup. Every private state and MIDI event byte is compared. Portable tests additionally check invalid LFO rate fallback.

The previous 136 percussion/pluck, 222 drone/acid and 312 pad/bus frozen-C vectors still pass, including complete private-state and PCM comparison. The full 43-case native instrument traces preserve the prior checkpoint's exact hashes. Six current WASM scenarios, the original R01 WASM F4 case, HTTP fallback, C0/Rust/R01 USB simulation and 24 frozen-source worker placement cases remain part of qualification. The portable workspace has 58 tests; formatting and clippy with warnings denied pass using pinned Rust 1.94.1. Host release costs retain all trials and establish no physical deadline claim.

```sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_voice_setup.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_voices_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_tonal_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_pad_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
```

## Remaining boundary and ticket status

R10 remains open for numeric per-block configuration assembly in `rust_tonal.inc`, `rust_pad.inc` and the drum/pluck adapters. Those currently gather C values and calculate block gains, modulation amounts and related derived coefficients before the Rust sample kernels. The next bounded checkpoint moves that arithmetic into the portable core while preserving association and call cadence.

C also retains parameter/application initialization and publication glue, MIDI channel mapping/dispatch, FM dispatch and harmonic target construction. Harmonic policy/voice-leading belongs to #8, FM6/allocator to #12, outer effects/mixing to #13, and job/buffer publication to #14/#19. The active vinyl/noise and pad chorus already reuse safe effects kernels. Physical stack/RAM/atomic/deadline qualification is still separate; this work performed no hardware I/O, flash, live-server, push or remote-ticket action.
