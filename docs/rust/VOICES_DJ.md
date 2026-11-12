# DJ snips: the remaining voice boundary

This completes the emulator implementation of the extra DJ/snippet voice scope in [R10 / #11](https://github.com/superposition/91/issues/11), after the named-family controls in [VOICES_PARAMETERS.md](VOICES_PARAMETERS.md). `EN_RUST_VOICES=1` now executes the real constructor, allocator, recipes, names, waveform generation, envelopes and mix/send arithmetic in safe `no_std` Rust. The original C bodies remain selectable with the flag off.

Despite the old `smp_note` and `EN_V_SAMPLE` names, these sounds are procedural synth voices. The 27 keys select 18 waveform kinds: horns, sirens, lasers, zaps, beeps, whoops, dub sirens, drops, a riser, rewind, crash, rave stab, bell and snare roll. The otherwise unselected legacy stab and unknown-kind fallback are also preserved and tested. No audio sample bank or performance ring is read.

## Representation and boundary

| Data | Size | Lifetime |
| --- | ---: | --- |
| DJ voice | 48 bytes | Existing four slots, 192 bytes total |
| Allocation age | 4 bytes | Existing wrapping unsigned word |
| Render configuration | 20 bytes | One scalar capture per block |
| Recipes and NUL-terminated names | Immutable | One Rust bank; C copies compile only in C0 |
| Execution diagnostics | 8 bytes | Two counters, private indices 72 and 73 |
| New voice state, table cache or heap allocation | 0 bytes | None required |

The voice representation has explicit three-byte padding and the same field offsets as C. It retains the oscillator phases, frequency/auxiliary values, sample time, envelope, two filter memories, held/mode/key/type bytes and allocation age. The bridge borrows these existing slots directly. It neither copies the bank into a packet nor stores a second runtime.

`smp_note` captures the key, octave and hold settings. Rust releases all active matching keys on note-off. Note-on selects the first free slot, or the first slot with the lowest unsigned age; increment and comparison retain the original wrapping behavior. Invalid keys do nothing. Rust returns before the caller can process another command, with no callback or retained reference.

`render_dj` captures level, send selection and the existing acid-bus gain ramp. One scoped call borrows the four voices, a local copy of the foreground RNG, the immutable sine table, and four disjoint output regions: left, right, reverb send and echo send. The bridge writes the RNG word back on return and publishes the active-voice count. It performs no C callback. Pad, FM and tank workers have separate buffers and RNG streams; this change does not edit the worker dispatch, join or quarantine seam.

## Ordering that matters

Rendering remains voice-major, then sample-major. Every active voice sample consumes one draw from the common foreground RNG, including tonal branches, zero level and a silent gain ramp. Branches retain the original binary32 expression association, table interpolation and integer conversion. Sample time increments before the sustained/one-shot gate test. Envelope release, natural expiry, state flushing, gain ramp and additive send writes occur in their original order, before sidechain processing.

Stop, panic and `en_init` deliberately retain the DJ bank and age, as the original engine does. The test checks this behavior rather than adding a new reset policy. Application-level changes to that behavior belong with application dispatch, outside this port.

## Evidence

[voices-dj-proof.json](voices-dj-proof.json) records the source identities, complete trace hashes, browser scenarios, worker placements and release-host timing trials. The immutable R01 source tree is verified against all 440 frozen file hashes before native qualification.

- **173 independent native cases:** all 27 keys × three octave settings × two hold modes; legacy/default waveforms; four-slot stealing and unsigned age wrap; natural/held release; invalid level/send clamping; zero level and an inactive null case.
- **41,810,684 Float32 output values** across four buffers and 128,000 calls, with all private voice bytes, age, RNG and active count compared. Maximum observed output magnitude is 0.873692989; every output is finite. Complete R01/Rust traces are identical, giving a zero null difference. Fresh C0 exports the same entire fixture as both.
- **Real browser controls:** all 27 keys and two polyphonic bursts reach Rust through the unchanged 24-export panel ABI. There are 36 accepted note-ons and 1,522,343 actual voice samples. The default snip level is 80. The scenario compares 2,651,292 stereo Float32 values and screen frames, including irregular render sizes, release tails, zero render and Stop. It also passes against the original immutable R01 WASM.
- **Whole-engine regressions:** 43 frozen native scenarios retain all earlier trace hashes over 161,688,822 integer PCM values, 209,246 MIDI messages and 86,265 clocks. Eight current WASM scenarios compare 48,431,888 Float32 values; HTTP adds 1,048,576, and the original R01 F4 comparison adds 2,557,800. Memory remains fixed at 32 MiB, with 24 public exports and no audio memory growth.
- **Active DJ worker placements:** 48 frozen/current-C0/Rust configurations compare full integer PCM while four held snips are audible at level 100. These cover serial, deferred, threaded and busy-worker fallback, reverb types, balances and predelay 0/100. USB simulation retains response hash `eff97ff8` in all three implementations.
- **Pinned portable checks:** Rust 1.94.1 formatting, 99 workspace tests and clippy with warnings denied pass. The portable suite includes the existing named-family/control/config fixtures and the two new DJ fixture tests.

The release host benchmark covers 19 representative recipes, three trials per implementation, 10,000 blocks of 128 frames with clears and periodic retriggers. Rust medians range from 15.6 to 85.9 ns per frame; Rust/reference ratios range from 1.13 to 1.82. These costs include the block adapter and one active voice, and show a host cost increase. They do not establish a physical MCU deadline. Exact per-case trials are retained in the proof alongside the earlier family memory/cost table.

```sh
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_dj_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_dj_workers.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh /absolute/R01/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
```

## Current C map

The named pad, drone/bass, acid, plain pluck, percussion, noise/vinyl and DJ voice kernels now have live qualified Rust paths. The earlier proofs cover their attacks, controls, fixed local state and numeric block assembly. This checkpoint adds the remaining DJ voice implementation without broadening application or synthesis policy.

| C or separately owned boundary | Ticket |
| --- | --- |
| Harmonic/register source selection, voice leading, compositional arc and target construction | #8 |
| Protocol/UI interpretation, application dispatch/init ordering, pending MIDI/FM callbacks and publications | #15; browser controllers #23 |
| FX/master/duck configuration, noise/vinyl setup and shared routing | #13 |
| FM6 operators, allocator and lifecycle | #12 |
| Worker publication, input lifetime, queue synchronization and retirement | #14/#19 |
| Physical stack/RAM/map/atomic/deadline and default-on qualification | Target integration |

C still captures scalars and pointers, calls the Rust bridge and publishes results. The default physical voice path remains C0 in this checkpoint. No hardware, live server, user workspace or remote issue was changed by this work.
