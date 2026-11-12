# Melodic scheduling checkpoint

This continues [R09 / issue #10](https://github.com/superposition/91/issues/10) after the [sample clock](TRANSPORT.md) and [drum scheduler](SEQUENCER.md). The Rust emulator now uses the original acid, pluck/motif and bass-line decisions through a safe `no_std` module. It also uses Rust for lane source selection, bar orchestration, queued grid FX dispatch, MIDI gate scheduling and transport reset/cancellation. The selectable C0 path remains available for differential playback; the ordinary hardware build still selects it.

The boundary is between deciding a note and applying it to a voice. [`melodic.rs`](../../rust/fm1-engine/src/melodic.rs) computes pitch, velocity, articulation and bounded actions. The C adapter copies the current state into an initialized value, calls the scoped [bridge](../../rust/fm1-jam-bridge/src/melodic.rs), copies the result back, then invokes the established C voice callbacks. A foreign pointer never outlives its call, and no callback occurs while Rust holds the borrowed state. The portable crate forbids unsafe code and needs no allocator.

## Data and limits

| Value | Bytes | Fixed content |
| --- | ---: | --- |
| Harmony | 92 | Twelve scale degrees, four chord tones, root/degree/octave and pool/pedal flags |
| Acid state / configuration / note | 64 / 56 / 28 | Sixteen offsets and octaves, three masks, loop/answer state; one returned note |
| Pluck state / configuration / note | 200 / 48 / 52 | Thirty-two recorded slots, sixteen motif intervals; one note and a sixteen-step UI publication |
| Bass state / configuration / note | 44 / 64 / 16 | Sixteen steps and history, four captured figure intervals; one returned note |
| Start state | 28 | Seven reset counters/positions |

The eight MIDI gates and sixteen delayed-hit slots keep their existing capacities. A grid tick returns exactly five optional FX opcodes in the established order. No additional event queue or persistent musical bank is introduced. Six execution counters at `web_rust_stat(22..27)` add 24 bytes only to the Rust-enabled image. C retains the musical storage during this stage; the structures above are bounded call values, not a new persistence format. Both sides assert their ABI sizes, and the structures use explicit integer and binary32 fields rather than foreign `bool` or enum layouts.

## Preserving the musical choices

Acid generation, capture, edits, all answer transforms, bar/loop synchronization and mutation use the common xorshift32 stream. The separate Songbook planner retains its LCG. A zero mutation amount still consumes the original per-step random draws. Acid advancement and possible mutation occur after the voice callback, matching the original order. Preview uses the same implementation as playback.

The pluck implementation keeps its sorted pitch ladder, recorded-slot replay and DejaVu decisions, style selection, leap/morph behavior, motif recording, inversion, alternate phrases, restatement and cadence. Its UI result is captured from the pre-update state, so the same generation is published before the note callback.

Bass decisions include manual steps, captured history, root/riff/figure/walk modes, movement, fifths, held pedal and octave decisions. The adapter captures the original table values, including the existing genre mapping. A pending bass note stays unfitted until the original fire callback folds it into the voice range; folding earlier would alter the pending state and later history even when a short audio comparison happened to sound identical.

The adapter preserves binary32 expression association, integer truncation, short-circuit RNG consumption, candidate order and callback interleaving. Returned decisions execute immediately at the same seam. Queued FX callbacks consume their own flags without changing the remaining arms, allowing their five opcodes to be selected together without changing dispatch order.

## Evidence

[melodic-proof.json](melodic-proof.json) records the artifact hashes, cases and observed coverage. The native runner compiles the real instrument against the immutable R01 source after verifying all 440 source hashes, then compares entire binary event/MIDI/PCM/state traces with the current Rust-enabled path. The adapter is shared; it does not duplicate the reference musical algorithms.

The 43 native scenarios retain the eight clock cases and nineteen genre/drum cases and add sixteen melodic cases. They exercise every acid length, bar/loop answers, manual and captured edits, lock changes, all pluck styles, DejaVu, bass modes and pedal, Hold/Idea, sections, FX, external-clock expiry and Start/Continue/Stop. Render sizes include zero, one, boundaries around 128 and irregular calls up to 1,024 frames. The longer melodic cases deliberately release DejaVu and play continuously across complete phrases. A trace audit requires observed motif recording, playback, inversion, alternate phrases and restatement, both answer clocks, bass-step playback and all eight occupied MIDI gates. Matching samples alone cannot satisfy that audit. The trace also retains the harmonic bar/sample countdowns and pending-change flag. Independent scenarios run with four workers by default (`PARITY_JOBS=1..8`), with each reference/Rust pair executing separately and comparing every byte.

Those traces cover 295,000 render calls, 161,688,822 integer PCM values and 209,246 MIDI messages, including 86,265 clock messages. All compared bytes match. The retained hashes summarize the full comparisons rather than replacing them.

The frozen acid oracle also checks 64 configurations over nine generations/mutations and 45 answer settings at all sixteen positions: 414,720 preview decisions, plus all stored masks/notes and final RNG words. Additional focused tests cover zero-amount RNG consumption, duplicate/full/tied MIDI gate admission, deadline expiry, all pending-slot cancellation and Start resets.

All four established WASM scenarios match 37,288,700 Float32 values directly, framebuffers, LEDs and save/restart, with fixed 32 MiB memory. The F4 legacy case also matches the original R01 WASM artifact and asserts that all six melodic execution counters advance. Factory and saved-v1 hashes remain `baeff971` and `28722449`; HTTP audio adds 1,048,576 matching values. The upload/save/readback/snapshot regression retains its `eff97ff8` response hash against C0, Rust and frozen R01. Pinned Rust 1.94.1 formatting, 36 workspace tests and clippy with warnings denied pass.

```sh
cargo +1.94.1 fmt --all --check
cargo +1.94.1 test --workspace --locked
cargo +1.94.1 clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_handoff.sh /absolute/R01/source
```

## Current call map and remaining work

| Work | Current implementation |
| --- | --- |
| Acid roll/mutation/answer/step/capture/edit/preview | Rust decisions; C `acid_fire` applies oscillator/envelope coefficients |
| Pluck ladder/style/recording/motif/capture/edit/step | Rust decisions; C `cm_trigger` applies voice/FM/envelope settings |
| Bass step/history/mode/note decisions | Rust decisions; C `bl_go_f`/`bl_note` apply pending notes to the voice |
| Sources, build/DJ/groove/wobble/answers, pad-stab and acid-link decisions | Rust; C copies results and invokes the existing callbacks |
| Bar/sample harmonic due/countdown and drift-change arming | Rust; C `chord_change`, `hr_bars`, `drift_reset` and retargeting still construct the harmonic state (R07 / #8) |
| Queued grid FX ordering | Rust opcodes; C effect callbacks run in order. Effect control and DSP kernels are separate R12 / #13 work |
| MIDI gate selection/deadlines and Start/Stop scheduler reset/cancellation | Rust; byte emission, voice gate/envelope cleanup, `notes_off` and application flags remain C |
| Songbook activation/copy and command routing | Earlier Rust boundary decisions and queue; C applies the selected application/storage effects |

Free-drift harmonic sample expiry now uses Rust too: it publishes one pending change and stops decrementing until dispatch; bar-based harmony clamps at zero while waiting for its bar callback. A focused test covers fractional expiry, the held pending state and bar clamping. The associated harmonic initialization and live parameter adjustments remain C as part of the harmonic/application boundary. C also retains voice envelope triggers, storage, callback dispatch and the surrounding tick/render traversal. The table/type and effects branches can replace their own remaining paths independently.

Live scheduling data literals also remain in C where the adapter captures them: `BL_ROOT`/`BL_RIFF`/`BL_FIG`, comp masks/genre selection, `SECT` section/build/wobble recipes and the `FILL_OK` genre mask. This port moves the decisions using those values, without claiming that every table has moved or duplicating an unreferenced Rust copy. The earlier drum module already supplies the live Rust ghost/fill templates, ratchet values, chain/slice and timing-rate rules, and Songbook drum-drift lengths; their older C literals serve the selectable C0 path. The local drone-envelope trigger table remains part of C voice application. Centralizing the remaining captured scheduler data is a separate cleanup from proving identical scheduling decisions.

This checkpoint proves emulator behavior for validated firmware inputs. Hardware exclusion/atomic support, ordinary-RAM/stack budgets and deadlines remain unqualified. No device was read, written or flashed. Issue #10 should only be closed against the remaining call map and its separate physical acceptance, not because a counter or module name says that all scheduling is complete.
