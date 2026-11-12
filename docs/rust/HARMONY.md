# R07 legacy harmony, arcs and register policy

[Issue #8](https://github.com/superposition/91/issues/8) already has the Rust
Songbook phrase planner. This follow-up moves the other active harmony system:
the legacy instrument's scale/chord helpers, weighted genre walk, authored
progressions, cadence/borrow decisions, pad/bass register targets, Naiwan arc
rotation and slow arc/section control. The emulator enables `EN_RUST_HARMONY=1`
inside its existing Rust build; `EN_RUST_HARMONY=0` retains the original C bodies.
Firmware defaults and the physical release gates are unchanged by this patch.

## The actual consumer boundary

| Existing entry or phase | Rust work | C capture/publication or dispatch |
| --- | --- | --- |
| `en_scale`, `en_chord_kind`, `en_voice_lead` | Pitch-pool expansion, degree wrapping, duplicate removal, fixed chord shapes and register search | Existing caller buffers and initialized-prefix writes |
| `scale_set`, `pool_set`, harmonic part of `regrow` | Natural/altered scale state, degree correction, history and loop reset | Voice reset/retarget callbacks outside the harmonic phase |
| `walk_pick`, `bor_decide`, `alt_apply`, `dom_apply`, `deg_dim` | Graph weights, cadence destination, no-ABA policy, diminished exclusions, secondary dominants and borrowed tones | Existing host diagnostic publication after return |
| `chord_change`, `chord_to` | Pending/rotating arc decision; authored or walk destination; chord/history header; following destination and alteration | `naiwan_prog`, `chord_to`, `pad_leave_stab`, `pad_retarget`, `bass_to`, trigger and drift callbacks in their original order |
| `pad_retarget`, `bass_pc` | Active-scale chord substitution, bounded register, open/drop-2 display, pedal targets and bass root | Four target/display writes and existing voice setup |
| `prog_take`, `en_prog_chord`, `en_prog_capture` | Packed slots, empty-slot behavior, timing-unit transition, minor dominant preview and history capture | Command posting, immediate stopped chord callback, byte-array publication |
| `naiwan_bing`, `naiwan_prog` | Four note/velocity results and eight-chord arc counter reset | Four queued pluck commands; factory arc bytes and application queue dispatch |
| `hr_bars`, `hr_bars_v`, `drift_reset`, `P_DRIFT` | Bar/seconds conversion, exact duration/RNG order and remaining-fraction update | Existing parameter capture and scalar publication |
| `control` arc/section phase | Slow mood target, timer, section smoothing and exact three-word expiry draw | Captured mood words, common RNG word and publication |

The Songbook planner's two phrase plans, 64 beat entries, authored v2 chart,
wrapping LCG and eight-note scale are unchanged. The legacy pitch pool has twelve
possible notes and its own common xorshift word. These are separate musical
systems with different limits; this patch does not coerce one into the other.
The deterministic planner corpus still passes in the workspace, and the actual
Hold/New Idea/save/restart WASM scenario still matches C.

The remaining C in this harmonic call map is application plumbing: parameter
and genre setters; command-byte assembly/posting and dispatch; factory recipe
pending notification; chord-name/text formatting; UI reads; existing scalar
capture/publication; and voice/MIDI execution after returned actions. Manual-pad dispatch still
invalidates the two next-walk sentinel bytes; drift progress shown by the UI
still uses the existing C publication arithmetic. Transport/sample countdowns
already use the separate Rust melodic helpers; C dispatches when they return. Public
engine signatures remain compatible. Those controller/presentation seams belong
to #15. Worker-owned pad targets remain inaccessible to foreground mutation until
the existing worker protocol confirms completion or retirement. A full coherent
UI/engine source capture is not newly claimed by these copied snapshots.

Harmony is reached from CPU0 render/drain and foreground contexts, including
UI preview calls. The native/WASM tests qualify serialized engine execution and
worker placement, not MCU interrupt serialization. In particular, the existing
`naiwan_bing -> en_note_v -> post` path makes the render consumer a command
producer too. Per-atomic exclusion alone does not serialize a whole foreground
queue push against a render interrupt. Root owns the #9 whole-publication IRQ
transaction guard and its physical qualification. No new claim of physical
producer serialization or a causal device-crash diagnosis is made here.

## Small phases preserve callbacks and randomness

A single monolithic state borrow would be wrong here. `chord_to` can reset drift
and trigger voices, both of which can consume the same xorshift stream. Posting
an authored progression while stopped can immediately reenter `chord_change`.
The adapter therefore captures numeric state, runs one Rust phase, publishes the
result and ends every Rust borrow before calling C. The next phase captures the
callback's resulting state and RNG.

For a walk change the order remains: arc decision and queued progression/bings;
clear alterations and choose the current destination; chord header/history and
dominant; retarget/voice callbacks and drift reset; choose the following chord;
decide its borrowed state; alter the current chord; publish any second retarget.
For a newly enabled stopped progression, the immediate chord callback still
runs **before** the requested timing unit is installed. The eight-chord arc reset
also remains after its original progression command posts. No callback runs
inside a live Rust state, voice or output borrow.

Short-circuit branches preserve draw count. Bar-based drift takes no draw;
seconds-based drift takes one. A normal walk always takes its weighted draw,
then a full varied loop can take the original extra 25% draw. Borrow decisions
and secondary-dominant tests consume words only on the original eligible
branches. Mood expiry consumes exactly three words even while stopped, but the
stopped arc position does not move. Songbook Hold freezes planner calls and its
LCG as before; it does not freeze this separate ambient xorshift stream.

## Layout, work bounds and compatibility details

| Numeric packet | Bytes | Purpose |
| --- | ---: | --- |
| `State` | 208 | Existing twelve-step arrays, chord tones, eight-slot/history/loop arrays, signed sentinels, counters and drift scalars |
| `Config` | 48 | Captured settings, pedal/play bits and step length |
| `Targets` | 32 | Four f32 targets and four i32 displayed notes |
| `Mood` | 20 | Four f32 controls and u32 timer |
| `Bing` | 32 | Four queued i32 notes and four velocities |

Generated C and Rust declarations assert every offset, size and alignment.
`harmony_read` explicitly clears the complete 208-byte C state before copying
fields, including its three padding bytes. No packet contains a pointer. The
adapters copy existing scalar state; they add no persistent harmonic state,
audio buffer, heap, expanded graph or delay allocation. Four diagnostic counters
use 16 bytes, indices 76..79. Packet size is a source-level copy/stack inventory,
not a measured maximum stack or linked firmware memory map.

The immutable graph remains 19 compact 30-byte rows (570 bytes), eight edges per
row. The major-mode alternate edges, borrowing/secondary percentages, Aeolian
steps, chord shapes/offsets, permutations, fallback weights and bell recipe total
818 source numeric bytes including the graph. The corresponding C numeric data
is 854 bytes; the difference is the bell recipe's 12 i8/u8 bytes replacing twelve
32-bit ints. This is a representation count, not a claim that all constants are
emitted separately by either optimizer.

The rich seven-note walk inspects at most seven candidates with eight compact
edges each (56 edge tests); other scales use at most twelve weights. A register
search visits the same 24 lexicographic permutations: 96 nearest-note operations
and 144 pair-collision tests. Cost is the sum of four absolute movements plus 40
per coincident pair. Strict `<` preserves the first equal-cost candidate. A pad
pedal can require a second 24-permutation search. Octave normalization uses
bounded integer correction instead of data-dependent repeated +/-12 loops.

Several small quirks are intentional compatibility, covered by the oracle:

- Root is always present, high pitch-mask bits are ignored, and unused C output
  tails are untouched. No Rust reference reads a caller's uninitialized tail.
- Negative roots use C's signed remainder. Register ties keep the note six
  semitones above. The raw bridge models partially overlapping `from`/`to`
  outputs through a copied alias map rather than aliased Rust references.
- The cadence counter wraps as u8. Diminished walking is excluded except for
  eligible borrowed flat-VII; authored destinations remain authoritative.
- Preview uses the currently altered scale minus the dominant raise, matching
  the old helper. Open displayed voicing and a pedal's close root voicing can
  differ. The bass pedal uses the key root while the shown chord still moves.
- Capture keeps byte 5's Play/unit/extension bits. A legacy custom degree above
  seven retains the original three-bit wrapping instead of inventing a new
  on-disk representation.

Public register inputs are qualified within +/-1,000,000 integer notes and the
normal bounded musical octave range; snapshots outside the declared count/index/range checks are rejected
without changing RNG or musical state. These limits cover every actual engine
and UI consumer. The safe musical crate forbids unsafe code; the bridge alone
performs aligned raw prefix copies and scoped packet borrows, retains no address,
and relies on each synchronous caller's initialized/disjoint packet spans.

## A concrete graph-to-voicing explanation for the talk

Take lofi in C major, currently ii, previous I, cadence counter 1 and no saved
loop. The ii-to-V edge adds 12 to V's base weight. The secondary-dominant pull
adds another 6. The no-ABA rule suppresses I. Candidate weights in degree order
are `[0,0,1,1,19,1,1]`, sum 23. A weighted result in V's interval `(2,21]` selects
V; the eligible flat-VII remains a borrowed possibility rather than a diminished
natural chord.

V's four pitch classes are `[7,11,2,5]`. From MIDI notes `[48,52,55,59]` in the
48..77 register, the first lowest-cost permutation yields `[50,53,55,59]`:
movements `[2,1,0,0]`, cost 3, no collision penalty. This is the same ordered
search used by the real pad targets, not a new harmonic scoring model. The
following destination is selected only after the current chord's callbacks have
consumed any of their original RNG words.

## Evidence and reproduction

Pinned Rust 1.94.1, strict binary32 C builds (`-ffp-contract=off`) and the immutable
R01 source manifest are used throughout. The focused driver includes the actual
C engine, not a duplicate reference model. It compares frozen C, freshly compiled
C0, the combined Rust engine, and independently selected Harmony-only Rust:

| Focused mode | Rows | Coverage |
| --- | ---: | --- |
| 0 | 987,136 | All 4,096 masks, eight signed roots, wrapped degrees, chord kinds and untouched tails |
| 1 | 12,000 | Partial `from`/`to` overlap, 24 offsets, zero-to-four tones, four registers and ties |
| 2 | 116,736 | All 19 raw genre graph rows, modes/keys/kinds, open/close and four pedal settings, active-scale preview and targets |
| 3 | 68,096 | Real walk callbacks, loops, altered/custom scales, forced tonic, RNG/MIDI order and u8 cadence wrap |
| 4 | 4,096 | Actual 1..383-frame control calls, timing units/drift edits, stopped/play mood and timer expiry |
| 5 | 6,656 | Empty/partial authored slots, unit changes, capture, pending/rotating arcs and queued flourish/reentrant drain |

All 1,194,720 rows match byte-for-byte. Walk mode observes 1,222 secondary
and 1,105 borrowed changes with zero diminished walk destinations. Its 136,942
MIDI callbacks match in position and content; authored/arc mode separately
preserves 13,566 callbacks and permits its 124 explicit diminished destinations.

The actual WASM next-chord scenario compares 3,475,080 Float32 values plus
framebuffers and LEDs and proves nonzero Rust chord-header/target calls with no
rejections. Full factory, saved v1, v2 Hold/restart, stored patterns, loaded FM6,
worker and effects scenarios also pass; counts/hashes, native worker placement
results, Rust checks and artifact limits are recorded in [harmony-proof.json](harmony-proof.json).

Run from this checkout (the frozen source path is external and immutable):

```sh
export RUSTUP_TOOLCHAIN=1.94.1
sh tests/rust_harmony_parity.sh ../baseline-2026-11-11/source
sh tests/rust_melodic_parity.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source extra-genres
sh tests/rust_effects_completion.sh ../baseline-2026-11-11/source
sh tests/rust_effects_workers.sh ../baseline-2026-11-11/source
sh tests/rust_fm6_workers.sh ../baseline-2026-11-11/source
sh tests/rust_worker_handoff.sh ../baseline-2026-11-11/source
sh tests/rust_handoff.sh ../baseline-2026-11-11/source
sh tests/rust_emulator_parity.sh
cargo test --workspace --locked --release
cargo test -p fm1-rt --features pi32v2 --release
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --locked --release
python3 tools/harmony_abi.py --check
node tests/host/emu_sequencer_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --harmony
node tests/host/emu_effects_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm
node tests/host/emu_fm6_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm
node tests/host/emu_worker_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm
node tests/host/emu_browser_test.mjs build/emu-parity/rust.wasm
```

The final Rust WASM is 2,404,436 bytes, 1,661 bytes below the preceding FX
checkpoint's recorded bundle. Fresh C0 is 1,941,589 bytes; both are rebuilt
byte-for-byte after final source comments. The build includes custom debug data,
so even source-column/comment changes can alter its artifact hash; size delta is
for the whole hosted artifact rather than a linked target text measurement.

A short WSL x86-64 sample (three process wall-time runs, including startup,
callbacks and trace hashing, no trace-file writes) gives these medians:

| Workload | Frozen C | Fresh C0 | Harmony-only Rust | Combined Rust |
| --- | ---: | ---: | ---: | ---: |
| 12,000 register cases | 18.826 ms | 19.634 ms | 18.016 ms | 19.566 ms |
| 68,096 walk/callback cases | 80.482 ms | 84.467 ms | 103.717 ms | 105.551 ms |

The walk harness is slower with copied state, checked phases and diagnostics.
These short host samples include unrelated engine callbacks and uncontrolled
workstation activity; raw samples and method are retained in the proof. They do
not establish audio-thread cost or MCU timing.

These are hosted native/WASM checks. No device, audio/MIDI hardware, compiler
backend, full firmware link, target stack, target cycles or deadline qualification
is claimed by this follow-up. Root's platform/resource/recovery checks remain the
physical acceptance gates; the C fallback and pinned recovery stub are retained.
