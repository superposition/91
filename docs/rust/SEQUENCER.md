# The drum scheduler as bounded decisions

This continues [R09 / issue #10](https://github.com/superposition/91/issues/10) after the [sample-clock port](TRANSPORT.md). The emulator now calls `fm1-engine::sequencer` for pattern density, step/time/slice/condition decisions, polymeter, fills/builds, ghost/probability/FX overrides, accents, humanization and ratchet/roll construction. Drum-drift timing and mutations, section-command decisions, delayed-hit deadlines and Songbook Hold/upload/queued-change boundary choices also run in Rust.

The portable module has no allocator, foreign pointers or unsafe code. The existing C voice functions execute its decisions in the same order. `EN_RUST_SCHEDULER=0` retains the original C implementation; the physical build keeps that default. Only emulator and explicit verification build flags enable the new path.

## Data and boundaries

| Representation | Size and purpose |
| --- | --- |
| `Lane` | 136-byte input captured by the render owner: masks, step/bar position, section/fill flags, parameters and binary32 timing/level values |
| `Hit` | 80-byte result: lane position/flags plus three 16-byte actions at most |
| Mutation row | Six 16-bit lane masks, 12 bytes; no new pattern bank |
| Pending-hit lookup | Sixteen signed sample deadlines: four melodic slots and two per drum |
| `Drift` | 28-byte scoped state/result, copied to the existing C counters |

The three-action bound covers an immediate triplet (fire, first delayed hit, second delayed hit) and an immediate ratchet (configure, fire, continue). Flam and ordinary hit plans need fewer actions. The caller first flushes any older pushed hits in the lane, as before, then executes the returned main-hit actions. It preserves side effects such as the clap's shared pending-velocity field. The scheduler creates neither an event queue nor another synthesis implementation.

The signed deadlines still live beside the existing C voice state. Rust chooses the earliest positive deadline and advances each slot; C fires the corresponding voice when Rust reports it due. Clock boundaries and the 128-frame delayed-hit floor still use the previously ported transport module. No callback occurs while a Rust pointer borrow is held.

The common musical RNG is xorshift32, distinct from the Songbook planner's LCG. The C adapter passes its current word into each scoped scheduling call and receives the final word back. Short-circuit conditions, candidate order, integer truncation and binary32 multiplication order are preserved. Voice firing does not consume this common stream, which permits one lane's decisions to be returned before its C dispatch while preserving the interleaving with other lanes and tick clients. Changing that voice contract later requires revisiting this boundary.

The C adapter still selects the source pattern ID at a bar boundary and supplies its mask and modification byte. Rust decides whether and how that step sounds. The surrounding C tick also orchestrates harmonic changes, acid-answer bars and performance-FX callbacks. Those are explicit remaining work, rather than claims hidden behind the new module name.

## Verification

[sequencer-proof.json](sequencer-proof.json) records hashes, counts and limits.

- Every 16-bit mask at all 101 supported densities matches the frozen C oracle: 6,619,136 combinations.
- Shift and Morph match the frozen C result and RNG word for 8,192 generated pattern/seed cases. Tests also preserve kick/clap patterns and protect existing snare backbeats.
- All 256 modification bytes across six lanes and sixteen positions fit the three-action bound. Deadline and phrase-boundary tests cover short/expired slots, Hold, pending edits and upload priority.
- Twenty-seven native full-instrument scenarios compare complete binary traces with the immutable R01 C source after verifying all 440 source hashes. The eight established timing cases are joined by nineteen current genre settings with changing chains/time/slices, normal and polymetric lanes, probability/density, active drift/mutation, Break-kit chops, held/queued fills, build/drop transitions and drum FX. Traces include integer PCM, MIDI messages, RNG, masks, pending-hit fields, section/drift state and clock/chunk counters.
- The three established WASM scenarios still compare 34,730,900 Float32 values directly, plus framebuffers, LEDs and save/restart. A fourth scenario enters the legacy instrument through its F4 key, changes piece/genre and stops/starts playback under irregular callbacks, adding 2,557,800 matching values. That scenario also matches the immutable original R01 WASM artifact. Factory and saved-v1 fingerprints remain `baeff971` and `28722449`. HTTP audio adds 1,048,576 matching values. New counters at `web_rust_stat(18..21)` verify that the actual lane, mutation, wait and boundary paths execute; they are development telemetry, not player-facing UI.

Pinned Rust 1.94.1 workspace tests, formatting and clippy pass on Windows. The native and no-std WASM executable comparisons use that same Rust version under Ubuntu-22.04, with the existing WASM clang wrapper and Node 22. CI runs the complete genre comparison and retains its JSON evidence. The existing upload/save/readback/snapshot regression also runs with the scheduler enabled.

```sh
cargo +1.94.1 test --workspace --locked
cargo +1.94.1 clippy --workspace --all-targets --locked -- -D warnings
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_sequencer_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_handoff.sh /absolute/R01/source
```

## Remaining R09 work

Legacy `acid_step`/acid mutation, `cm_step`/motif and bass-line note generation still contain C sequencing decisions. Some harmonic/bar counters and FX orchestration, source-ID selection and Start/Stop voice cleanup also remain C. The borrowed pack/session/planner work and these drum decisions do not constitute a complete port of every musical scheduling path. The whole-ticket acceptance therefore remains open.

The new request/results use bounded stack storage; four diagnostic counters add 16 bytes to the Rust-enabled image. This is emulator evidence with fixed 32 MiB memory, not a hardware deadline, stack or ordinary-RAM qualification. No physical firmware was flashed and no hardware interaction was required.
