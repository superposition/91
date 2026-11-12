# Porting the clock without changing the groove

This is the first integrated slice of [R09 / issue #10](https://github.com/superposition/91/issues/10). The browser build now uses the safe, allocation-free `fm1-engine::transport` module for the 24-PPQN subdivision, sample phase, external-clock timeout, render chunk limits and slow-control cadence. The C musical event handlers and synthesis remain in place. The physical build still selects its existing C path.

## Representation and call order

`Transport` has four fields, all four bytes wide, in a 16-byte `repr(C)` structure:

| Field | Meaning |
| --- | --- |
| `pulse_left: f32` | Samples remaining to the next internal clock pulse |
| `pulse: u32` | Position 0..5 within a musical sixteenth |
| `external_age: u32` | Render calls since the last external-clock message |
| `control_acc: u32` | Frames accumulated since the slow controls last ran |

These replace the same four C scalars without enlarging that state. The existing four-byte `step_len` cache remains available to C DSP. Three transitional diagnostic counters add 12 bytes to the Rust-enabled build; they are exposed only through the existing `web_rust_stat` export at indices 10..12. Browser memory remains fixed at 32 MiB. This is not a measured physical-device RAM or timing qualification.

The transitional C allocation is zero-initialized and modified only by scoped Rust calls. The render owner serializes every call. The bridge borrows the state only for that call and never keeps a pointer or calls C while borrowing it. Both languages assert its size and alignment. The portable module forbids unsafe code through its crate; raw-pointer conversion is restricted to the separate bridge.

At a due pulse the caller first asks whether the subdivision is zero, dispatches the existing musical tick and MIDI clock, then asks Rust to advance the subdivision. It schedules the next period **after** those callbacks, using the possibly changed tempo. This preserves the original callback order rather than assuming a tick cannot affect later decisions.

The remaining calculation deliberately uses binary32 values in the same order as the original C. In particular, DX Songbook coalescing evaluates `(6 - pulse) * step_len * (1 / 6)` from left to right. The next musical sixteenth can split a chunk at any sample; delayed notes retain the existing 128-frame minimum where possible. Slow controls accumulate short chunks and receive all elapsed frames when the 128-frame threshold is reached.

An external-clock message resets its age. The timeout remains **101 render calls**, including zero-frame calls, rather than becoming a new duration in milliseconds. Continue retains the six-pulse subdivision; Start resets it. These details can sound like implementation trivia until an apparently harmless change moves a note or increases render work.

## Evidence

[transport-proof.json](transport-proof.json) records exact artifact hashes and the results. The native adapter includes the selected instrument's unedited `amb.c` and runs its real command drain, `en_render` and MIDI path. It does not reproduce the timing algorithm in test code. Against the immutable R01 source, all 440 source hashes verify before compilation.

Eight native cases use fixed 256-frame calls or irregular 0..1,024-frame calls, with tempo limits/changes, Stop/Start/Continue, external pulses and expiry, legacy swing/humanization/ratchets/polymeter, and DX Songbook Hold, key/form changes and New Idea. The script compares complete binary traces byte for byte: integer PCM, MIDI events and render position, the four clock counters, step length, playing/external state, sixteenth/step position, chunk count and song revision. The JSON fingerprints summarize these comparisons; they are not a substitute for the full trace comparison.

The existing three complete WASM scenarios still match **34,730,900 Float32 values** directly, along with framebuffers, LEDs and save/restart. The factory and saved-v1 PCM fingerprints remain `baeff971` and `28722449`. HTTP audio additionally matches 1,048,576 values. Each scenario asserts that Rust pulse, chunk and control counters actually advance. No command drops, planner rejections, C planner fallbacks or memory growth occur in these scenarios.

Five focused portable tests cover every supported tempo against 121 frozen-C binary32 values, Start/Continue subdivision behavior, clock timeout/reset/wrap, accumulated controls and the interaction between early clock boundaries and delayed notes. Workspace tests and clippy pass on pinned Rust 1.94.1 on Windows; native and WASM executable comparisons also pass with Rust 1.94.1 on Linux.

```sh
cargo +1.94.1 fmt --all --check
cargo +1.94.1 test --workspace --locked
cargo +1.94.1 clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_transport_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
```

Without the optional R01 path the native script compares the checkout's selectable C0 and Rust paths, and always checks the frozen tempo corpus. CI runs this comparison in addition to the existing complete WASM comparisons. The retained local traces are native diagnostic files, not a public storage or wire format.

## Work remaining in R09

The subsequent [sequencer port](SEQUENCER.md) moves drum decisions, drift/mutation, ratchet and delayed-hit timing, and Songbook boundary choices into Rust. The [melodic checkpoint](MELODIC.md) adds acid/pluck/bass decisions, source selection, bar/FX orchestration, free-drift expiry and transport scheduler cleanup. Its current call map describes retained harmonic construction, voice application and application effects. Hardware exclusion/atomic backends, memory budgets and render deadlines remain unqualified for these Rust slices; this document and its proof retain the earlier clock checkpoint evidence.
