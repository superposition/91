# Songbook snapshot and staging checkpoint — R08

This implements an emulator-qualified part of [ticket #9](https://github.com/superposition/91/issues/9): one copied Songbook view and one staging-buffer lease. Full application snapshots, physical producer routing and target interrupt-exclusion qualification remain open. No hardware firmware was flashed.

## One view per frame

The Songbook header, performance body, four detail pages and knob row now consume one captured generation. The 220-byte view contains the ten-byte settings, display state, revision, ten option bytes, five bounded 32-byte strings and two eleven-byte voice names. It contains no pack pointers. The UI-owned tempo remains stable in the foreground project during drawing.

`en_jam_snapshot` byte-initializes the entire C capture, including padding, before filling it. C asserts its 220-byte size and the revision/title/voice-name/tail offsets (32/36/196/218). The Rust ABI copies exactly 220 initialized bytes, supports in-place input/output, retains no foreign pointer and rejects invalid lengths/null pointers before writing the output.

`SnapshotCell<T: Copy>` uses one acquire/release gate for both reading and publication. Each attempt performs one compare-exchange and one bounded copy, returning immediately on contention. Payload reads never race with writes; this is not a seqlock over non-atomic bytes. A private RAII gate also releases on unwinding. `Sync` requires `T: Send`, and no reference into the cell escapes. A contended UI handoff leaves the previous frame displayed.

The generic cell safely supports concurrent publishers/readers. This transitional ABI requires one serialized source caller so its read corresponds to its own publication. The emulator's single execution context excludes source mutation during C capture. **The destination gate does not protect C engine state.** A physical backend must first provide a bounded source capture under audio exclusion, measure its cost and review every remaining direct global read. `EN_RUST_HANDOFF` is enabled with the browser Rust build; the physical build does not enable it.

## One staging lease

Rust now arbitrates the actual Songbook transfer state formerly held in `ju.owner`:

| Current state | Allowed next state | Release condition |
| --- | --- | --- |
| Idle | Receiving or readback | A foreground transfer claims the buffer |
| Receiving | Awaiting activation or idle | Valid audition, abort, timeout or failure |
| Awaiting activation | Idle | `en_jam_busy()` becomes false after render finishes reading |
| Readback | Idle | Last chunk, abort, timeout or failed save verification |

The existing C buffer remains the only 3,840-byte staging payload. This is arbitration over that external buffer, not Rust memory ownership of its allocation. C must serialize protocol/legacy-backup use and respect the lease before touching its bytes. The foreign transition API is therefore unsafe and documents that contract. The retained upload pointer remains a C runtime boundary until the application/render state migrates.

Safe Rust callers can claim a non-clonable guard and move it from receiving to activation. Dropping the guard releases the lease. The C path uses checked atomic transitions, retains activation even beyond the receive timeout and refuses new receive/readback work while it is held. Saved-song loading claims the same buffer before loading and releases on invalid/missing packs; saving holds readback through verification. Wire IDs, frame bytes, checksums, token handling and stored formats remain unchanged.

## Storage and limits

| Item | Bytes |
| --- | ---: |
| Copied Songbook view | 220 |
| Snapshot cell including its gate | 224 |
| UI cached view | 220 |
| Staging arbitration state | 4 |
| Five telemetry counters | 20 |
| Total added static storage in this emulator checkpoint | 468 |
| Existing staging payload, retained without duplication | 3,840 |

The C capture also uses a 220-byte temporary; Rust performs scoped byte copies. Physical stack, RAM and interrupt-duration budgets still require target measurements. The emulator retains its fixed 32 MiB memory and 24 browser exports. Telemetry indices 13–17 report publications, copies, staging claims, activation transitions and rejected transitions; these are developer diagnostics and are not displayed in the player.

## Evidence and reproduction

The checked result is in [songbook-handoff.json](songbook-handoff.json). Rust 1.94.1 native workspace tests and Clippy pass. The snapshot test transfers 100,000 patterned 220-byte values between real threads without torn observations. Another test makes 100,000 competing receive/readback claims while retaining activation, with no overlapping lease. Deterministic contention checks ensure attempts return without overwriting output. ABI tests cover in-place copy, invalid input and stale/illegal transitions.

The native integration test exercises the real C/USB/request paths with Rust enabled: malformed/incomplete/duplicate chunks, CRC failures, abort, receive timeout, activation retained beyond 30 seconds, blocked buffer reuse, save/readback/reboot and invalid/missing saved packs. It verifies every snapshot field, bounded strings and independence after a subsequent upload; the actual UI cache shows the new generation. Current C, Rust and independently frozen R01 C all produce response-byte hash `eff97ff8`; all 440 frozen source hashes are verified before compiling that independent reference.

All three full-instrument differential scenarios match 34,730,900 stereo Float32 values, sampled framebuffers, LEDs and save/restart behavior. The HTTP path matches another 1,048,576 values. A separate WASM check observes 134 snapshot publications/copies across the performance screen and four detail pages. No browser interactive/audio-device/MIDI QA or physical exclusion timing is implied by these source/native/Node results.

```sh
cargo +1.94.1 test --workspace --locked
cargo +1.94.1 clippy --workspace --all-targets --locked -- -D warnings
cargo +1.94.1 check -p fm1-rt --target wasm32-unknown-unknown --locked
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
# Linux host C compiler; optional R01 source directory adds independent verification:
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_handoff.sh /absolute/R01/source
```

The CI emulator job runs both scripts. The hardware flag remains disabled and R08 remains open for its broader application/backend qualification.
