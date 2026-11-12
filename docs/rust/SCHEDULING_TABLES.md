# Scheduling recipes checkpoint

This completes the numeric scheduling-data boundary after the [melodic checkpoint](MELODIC.md) for [R09 / #10](https://github.com/superposition/91/issues/10). The live emulator now reads its bass figures, comp choices, section/build/wobble recipes and allowed-fill masks from Rust. The original C literals remain selected by C0 and the default physical build.

[`scheduling_tables.rs`](../../rust/fm1-engine/src/scheduling_tables.rs) holds the compact immutable data and the bass/comp selection rules. The bridge exports one immutable `FM1_RUST_SECTIONS` array containing nineteen nine-byte rows, plus nineteen `FM1_RUST_FILL_OK` bytes. C reads those process-lifetime views directly using its existing expressions. The nine-field C/Rust section layout has size nine and alignment one, asserted on both sides. No mutable source-copy protocol, allocation, runtime initialization, extra cache or diagnostic counter is added.

The bass adapter passes its existing mapped genre and validated mode to Rust, which supplies the root mask, selected riff/figure mask and four sign-extended intervals in the existing 64-byte configuration. Genre mapping still happens at the original seam; applying it again would change several genres. Comp lookup preserves Songbook's one-bit mask, Auto and all seven explicit patterns. The immutable payload is 448 bytes before compiler alignment: 190 section/fill bytes, 206 bass bytes and 52 comp bytes, replacing their corresponding active C data.

The earlier drum port already supplies the live Rust ghost/fill templates, ratchet values, chain/slice and timing-rate rules, and Songbook drum-drift lengths. Its older C copies serve the selectable C0 path. This checkpoint therefore adds the remaining live data rather than creating another unreferenced implementation of those rules.

## Verification

[scheduling-tables-proof.json](scheduling-tables-proof.json) records the final artifact hashes and evidence. A new oracle compiled against the immutable R01 source covers every section field and fill mask, all 323 genre/bass-mode combinations and 342 comp/Songbook combinations. The same nineteen independent C fingerprints are checked by the portable test and both native executable paths. This checks values even when a particular playback scenario does not select them.

The full forty-three native scenarios again compare complete event/MIDI/PCM/state traces after verifying all 440 reference source hashes: 295,000 calls, 161,688,822 integer PCM values and 209,246 MIDI messages including 86,265 clocks. The execution audit still observes motif recording, playback, inversion, alternate phrases, restatement, bar/loop answers, bass steps and eight occupied MIDI gates. All four WASM scenarios compare 37,288,700 Float32 values plus framebuffers, LEDs and save/restart; the F4 legacy case also matches the original R01 WASM. HTTP audio adds 1,048,576 values. Snapshot/staging/USB-response regressions still match C0, Rust and frozen R01. Rust 1.94.1 formatting, 37 workspace tests and clippy with warnings denied pass on this isolated scheduler branch.

The existing `tests/rust_melodic_parity.sh` and CI entry include the new frozen table oracle automatically. No feature flag, callback order, RNG draw, binary32 operation, queue capacity, render cadence or diagnostic index changes.

## R09 implementation and the C seam

The emulator scheduler implementation can be treated as complete with the delivered clock, drum, melodic and table checkpoints. The remaining C is explicit application/voice/harmonic plumbing:

- C stores the musical state, captures scoped inputs and applies returned fields/actions. It assigns or flushes pending-note slots and invokes immediate/delayed synthesis callbacks in their original order. Rust decides notes, articulations, deadlines, ratchets, phrase/section boundaries, transport resets and bounded action choices.
- C applies the selected Songbook activation/copy, parameter and command effects; platform MIDI emits the selected bytes. UI publications still use the established application boundary.
- Harmonic construction, `hr_bars`, `drift_reset`, progression/scale selection and live harmonic-parameter adjustments remain harmonic-engine work under #8. The bar/sample due countdown and dispatch decisions are Rust.
- C applies voice oscillator/filter/envelope coefficients, gate/tail/Start/Stop cleanup and drone-envelope triggering under #11/#12. Performance FX callbacks and control/sample processing belong to #13; factory names/packed records belong to #5.

These boundaries do not claim that the application or voices have moved. Hardware backend/atomic/stack/RAM/deadline qualification remains separate, with the physical build selecting C0. No hardware interaction, live server change, push or issue closure was performed by this checkpoint.
