# DSP job lifetime and staged outputs

[R13 / issue #14](https://github.com/superposition/91/issues/14) now has a real
Rust-controlled job boundary behind `EN_RUST_WORKER`. It preserves the retained
worker placements and DSP order: optional bass/acid, reverb tank, pad, FM6.
Physical stop mechanics and the MCU atomic backend remain separately qualified
by the platform work; a timeout alone never releases a DSP buffer.

## Descriptor, stages and ownership

`fm1-rt::job::JobMailbox` owns the copied, immutable 32-byte job descriptor,
generation/state word and stage publication. The descriptor contains frame
count, predelay read position, FM enable, FM/pad write positions, bass placement,
genre and integer/float FM selection. The last two values are snapshots because
their public UI setters are immediate; a check followed by a write would leave
an IRQ window. Other worker configuration inputs are render-owned and remain
unchanged until ownership returns. The worker receives a generation encoded by
value as its opaque callback context, rather than a pointer to an overwritten
future descriptor.

The mailbox occupies 96 bytes, aligned to 32 bytes, with separate control and
stage cache lines. One claim changes Ready to Running and copies the descriptor
after acquire. An unfinished lease's drop leaves Running. Each completed stage
releases only its own outputs after all kernel borrows have ended; the remaining
worker state stays leased. Completion is released after the C callback returns.
Stale generations cannot claim, publish, cancel or reclaim a newer job. A
29-bit generation never wraps: exhaustion retires asynchronous dispatch into
the serial path rather than reusing an old token.

```mermaid
sequenceDiagram
  participant M as Main render
  participant Q as Rust mailbox
  participant W as Worker
  participant P as Platform
  M->>Q: Copy inputs, prepare generation
  M->>P: Submit callback with generation
  P->>W: Invoke callback
  W->>Q: Claim once with acquire
  W->>Q: Release bass and acid stage
  M->>Q: Acquire stage before reading outputs
  W->>Q: Release tank stage
  M->>Q: Acquire tank before mixing return
  W->>Q: Release pad, finish FM, release completion
  W-->>P: Return callback
  M->>P: Join
  P-->>M: Confirm returned or permanently stopped
  M->>Q: Acquire completion or retire stopped job
  M->>M: Publish meters and drain pad MIDI
  M->>Q: Reclaim completed descriptor before next prepare
```

The existing audio arrays and voice/effect state remain in their established
pool/BSS regions. Scoped DSP bridges borrow bounded buffers only while their
stage owns them. The pad/FM FIFOs retain their disjoint producer/consumer spans,
and the tank's predelay reads remain separated from current input writes.
Descriptor synchronization does not create an exclusive reference to a whole
shared ring, and does not by itself own foreign C memory: the reviewed bridge
contracts and confirmed worker lifetime protect that memory.

Thirty-two bytes stage the worker-written pad/bass/acid levels and pad note/amp
fields. Only the joined main owner publishes public UI meters. This matters
when an unconfirmed worker outlives a render call: UI reads do not touch its
private in-progress writes. The original 16-message pad MIDI queue is retained.
Its main producer runs before dispatch; its worker producer runs during the
pad stage. Main now flushes after confirmed join, so those accesses never
overlap. Frozen serial packet/callback ordering is preserved; threaded/deferred
ordering is made deterministic instead of depending on the old queue race.

No allocator or additional audio/voice bank is introduced. The named static
storage delta is 164 bytes: 96-byte mailbox, 32-byte diagnostics and 32-byte
private meters, plus the enlarged C descriptor and two lifetime scalars, less
the three removed volatile stage flags. This is a source-layout count, not a
linked MCU memory map. Existing fixed voice, FIFO, staging and MIDI limits stay
unchanged. Diagnostic indices 56..63 report prepared jobs, claims, released
stages, completions, refused/serial dispatch, retirements, stale operations and
quarantines through the existing statistics export.

## Refusal, loss and unknown ownership

The platform contract is explicit:

| Result | Meaning | Main may do |
| --- | --- | --- |
| submit 0 | No callback was queued or started | Claim that generation inline once |
| join 1 | The callback has returned | Also acquire Rust completion before reuse |
| join 0 | Current execution and all nested DSP callbacks are permanently stopped | Retire that generation, discard results and recover |
| negative join | Execution/ownership is still unknown | Quarantine; write silence only to separate main output |

A stage wait has at most 65,536 polling attempts before calling the platform's
bounded join. A missing stage publication can therefore fall back to a genuine
completed join. A stale backend completion cannot override the Rust generation
and completion checks. Init/attach require serialized boot/no outstanding job;
active-job attempts are rejected. The host-only reseed hook also refuses an
outstanding worker.

While quarantined, render does not consume queued controls, advance musical
state, prepare another job, reset state or borrow worker-owned outputs. It
zeros only the caller's main/USB output spans. A later bounded join must confirm
completion or permanent stop before recovery accesses those buffers. Recovery
discards partial pad MIDI, uses the existing all-notes-off operation so a dropped
note-off cannot leave an external note held, and restores the affected audio
buffers/effects. Permanent retirement disables future asynchronous submissions;
subsequent jobs execute serially. The old job is never replayed over partly
advanced DSP state. Silence/clock freezing and dropped results are explicit
fault behavior, not normal musical timing changes.

The C fallback remains selectable with `EN_RUST_WORKER=0`; its historical
volatile-stage/queue assumptions are not claimed as the new synchronization
proof. The default physical firmware is not enabled merely by a hosted pass.

## Software evidence

Use pinned Rust 1.94.1 and the immutable R01 source checkout:

```sh
RUSTUP_TOOLCHAIN=1.94.1 cargo test -p fm1-rt --locked
RUSTUP_TOOLCHAIN=1.94.1 cargo test -p fm1-rt --features pi32v2 --locked
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_worker_handoff.sh ../baseline-2026-11-11/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source extra-genres
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_effects_workers.sh ../baseline-2026-11-11/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_fm6_workers.sh ../baseline-2026-11-11/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_melodic_parity.sh ../baseline-2026-11-11/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_handoff.sh ../baseline-2026-11-11/source
RUSTUP_TOOLCHAIN=1.94.1 sh tests/rust_emulator_parity.sh
node tests/host/emu_worker_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_fm6_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_effects_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_browser_test.mjs build/emu-parity/rust.wasm
```

The six runtime tests cover one-shot inline dispatch, stale tokens, dropped
leases, quarantined completion, permanent stopped retirement and exhaustion.
One test concurrently publishes 100,000 32-byte descriptors and three separate
output stages, checking every output word after acquire.

The native harness includes actual DSP dispatch, render, MIDI and UI publication
paths. Twenty-four valid cases cover serial/deferred/pthread/busy execution,
missing stage publication and stale callback replay, with both work placements
and legacy/v2 keyboard/bass sources. They compare full PCM, fixed USB output,
MIDI packet/callback order and public-meter bytes against frozen serial C and
fresh serial C0. Twelve fault cases cover loss at bass/tank/pad, unknown ownership
followed by completion or stopped retirement, and a false backend completion.
The tests require silence with bounded main/USB writes, unchanged leased state
and inputs during quarantine, deferred queued controls, and continued serial
DSP after permanent retirement.

Fault tests pause only at a real stage boundary after kernel borrows end, then
request cooperative stop and `pthread_join` before returning stopped. They never
declare a live writer dead. This establishes the software ownership protocol;
it does not claim a physical asynchronous stop inside a DSP instruction.
`worker-handoff-proof.json` records the final combined software qualification
of implementation merge `6a12561a9928e11fa355dab65b252f7e6099387a`, including the
canonical codecs, wire adapters, voice configuration and atomic facade. The
workspace passes 99 tests; the opaque PI32v2 atomic ABI test backend passes 14
runtime tests, including the six job tests. Format, warnings-denied Clippy and
release `no_std` checks pass with Rust 1.94.1.

The handoff harness passes 36 cases and 8,353,800 frames. Separate full-render
regressions pass 23 effects cases, 48 effects worker placements and 32 FM6
worker placements. The expanded melodic regression matches 43 frozen/fresh-C0/
Rust traces: 80,844,411 frames and 209,246 MIDI messages, with recorded/motif/
alternate/inverted/restated/answer paths and all eight MIDI gates exercised.
These full-render scripts explicitly enable `EN_RUST_WORKER=1`; isolated direct
operator/configuration oracles retain their narrower kernel scope. USB staging,
busy/readback/save/reboot checks retain response hash `eff97ff8`.

The final 2,397,205-byte WASM artifact has SHA-256
`2445a51f7aabddc3bdff79dcb320720967dbb95248b7192d957b7f9dbdac0c68`.
It retains 24 exports and 32 MiB memory. The expanded suite compares 47,059,496
Float32 values with fresh C0; HTTP boot adds 1,048,576 values. Independent frozen
C checks also match the worker, loaded-user-FM6 and effects scenarios against
that exact artifact. The worker scenario compares 1,278,900 values, has nonzero
peak 0.47123 and fingerprint `f99f9451`, and observes 2,710 prepared/claimed/
completed inline jobs plus 5,420 stage publications, with no retirement, stale
operation or quarantine. Browser API/lifecycle mocks pass; this is not manual
desktop/mobile/audio/MIDI qualification.

## Remaining platform boundary

The C render traversal still chooses placements, calls the DSP stages, mixes
their released spans and drains foreground callbacks. Platform submit/join
implementations must provide the return/stop guarantees in the table above;
the Rust mailbox cannot manufacture those guarantees for foreign execution.
The actual stop/cache/IRQ implementation and stopped atomic-lock-owner recovery
remain platform work. The opaque hosted ABI test validates the interface, not
the MCU hardware backend. See [MCU_ATOMICS.md](MCU_ATOMICS.md) and
[PI32V2.md](PI32V2.md) for separately scoped compiler/backend evidence.

Physical link/map/pool/stack, cache visibility, stopped nested callback borrows,
cycle/deadline and audio output qualification remain required on the integrated
firmware image. No physical firmware flag or hardware acceptance is inferred
from this software report. The retained C0 path also remains outside the new
Rust lifetime/synchronization proof.
