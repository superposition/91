# R12 outer mixer follow-up

This follow-up to `4bd423b` moves the remaining **outer mixer arithmetic** into
`fm1-engine::effects::mix`. C still generates voices, traverses the render stages,
and supplies the existing storage. No audio payload buffer, cache, allocator,
flag, statistic slot or WASM export is added.

## Call map

| Stage | Rust arithmetic | Retained C plumbing |
| --- | --- | --- |
| Chunk setup | Reverb input gain and kick duck depth | Capture the current scalar arguments before starting the job |
| Plain pluck | Move its current stereo contribution into the target bus | Select whether that routing is active |
| Pad + FM | Add stereo samples in pad-then-FM order, pan gains and dry/send combinations | Read exactly the previous pad-ring span, split at wrap; supply already-produced FM working samples |
| Sidechain/drums | Existing Rust sidechain, gate, differential gain/crush, plus differential drum-to-bus routing | Voice generation and before-buffer copies |
| Bass + acid | Ducked/gated pan gains, accumulation and send combinations | Join/fence their stage before exposing the output spans |
| Track bus | Existing Rust filter/phaser | Decide whether the bus is active |
| Echo/send/dry | Echo input combination, tape/Echoplex, predelay-send construction, dry scaling and tank duck scaling | Commit the prepared predelay inputs with the original `push`, in sample order |
| Tank return | Add the ducked tank outputs into the stereo working mix | Join/fence the tank stage first; retain the subsequent performance-effect traversal |
| Performance/dynamics/output | Existing Rust performance/tone, compressor/limiter and conversion | Traverse the established stages and publish output |

`rust_mixer.inc` contains only pointer/config marshalling, pad-span splitting,
echo-state capture/copyback and ordered shared-predelay commits. The core receives
six distinct foreground working spans: left, right, reverb send, echo send, bus
left and bus right. Both sources can select the same accumulator without passing
aliased mutable Rust references: routing occurs inside the core. Pad and FM
contributions are still added separately in their original order. The new
cancellation test preserves `(1 + 2^24) - 2^24 == 0` in binary32, rather than
regrouping the contributions.

The remaining C arithmetic in this render slice is at the **voice/storage
boundary**, not hidden as an unported outer mixer stage:

- `pad_bus` still computes the pad's source filter/output ramp and dry sum.
- `render_fm` still prepares the delayed FM source level, voice ramp and its
  stereo/send working outputs.
- Intrinsic additions inside `render_cm`, `render_dj`, `render_hat`,
  `render_clap`, `render_snare`, `render_perc` and `render_drums` remain with their
  C voice kernels, for the voice tickets (#11/#12).
- `push` still quantizes and writes the shared predelay. Clearing/copying work
  buffers, `mx_begin`, `pad_publish`, ring indices, stage submission/join/fences,
  retirement/reset orchestration and render traversal remain C.

C parameter application still supplies the source settings/scalars (including
`rev_send`, `echo_send`, `pl_decay_t`, `echo_fb` and track pan/send values).
Migrating that application layer is separate from these render combinations.

The musical core stays `no_std`, allocation-free and `forbid(unsafe_code)`.
`MergeConfig` is a 32-byte numeric `repr(C)` packet and `EchoSends` is 28 bytes;
matching C/Rust size assertions apply. Scoped unsafe bridge code checks no
ownership by itself: the documented alignment, initialization, disjointness and
stage-lifetime requirements are obligations of the reviewed C adapter.

## Sharing the rings without borrowing the whole allocation

The pad ring has 512 frames; the foreground consumes the span 256 frames behind
the worker's current write cursor. Each chunk is at most 256 frames. Those spans
are disjoint under circular rotation. `rust_mix_pad_fm` splits the readable span
at the ring end and constructs immutable Rust slices covering **only each exact
consumed span**. It never creates a reference to the full shared pad ring.

Echo processing now runs as a block. It consumes each original reverb-send value
and reuses that same `rsend[j]` slot for the computed predelay input. These private
working slots have no later send consumer. The echo feedback, sample ordering,
dry arithmetic and ring state transitions remain in the original per-sample
order. C subsequently calls the original predelay `push` in increasing sample
order. No extra audio array or Rust reference to the shared predelay is formed.

Delaying those writes until the echo block returns cannot affect the current
tank job: in a 1,579-word circular predelay the retained read spacing is
256..1,322, while both the read and write spans are at most 256 words. Rotating
the write cursor to zero, the read span begins at `1579 - spacing`, at least 257,
and ends no later than 1,579. It cannot intersect `[0, frames)`. The new invariant
test covers every retained spacing and chunk length. Worker differential tests
add actual serial/deferred/pthread/busy execution at **both spacing endpoints**.
The worker's tank-only Rust borrow still excludes all 1,579 predelay words.

The previous echo adapter captured 56 bytes of state and 48 bytes of config,
then copied 56 bytes of state back, **per sample**. The real render now does that
**once per block**, plus the 28-byte send packet and a six-pointer working-span
table. Pad mixing needs at most two calls per block; other new mixer kernels use
one. No new per-sample scalar packet is added. These are source packet counts,
not measured memory traffic, stack high-water marks or PI32 cycle estimates.
The tank-return kernel introduces one extra in-place stereo write phase before
the existing FX traversal (8 bytes written per frame, subsequently read by that
traversal). This uses the existing mix arrays and avoids expanding a per-sample
FFI call. It is a bounded tradeoff against moving the echo's 160-byte capture and
copyback from every sample to every block; exact memory traffic is unmeasured.
The separate direct-sample echo entry remains for the frozen impulse harness.
Other existing per-sample FX adapters and their qualification limits are as
recorded in `EFFECTS.md`.

## Evidence and reproduction

Use pinned Rust **1.94.1**, Node **22.14.0**, and the existing WASM clang wrapper.
The baseline path below is the immutable R01 checkout; its 440 source hashes are
verified before native reference compilation. Run test groups with separate
output folders as provided by the scripts.

```sh
export RUSTUP_TOOLCHAIN=1.94.1
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check -p fm1-types -p fm1-formats -p fm1-engine -p fm1-rt --target wasm32-unknown-unknown --locked
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source extra-genres
sh tests/rust_effects_workers.sh ../baseline-2026-11-11/source
sh tests/rust_emulator_parity.sh
node tests/host/emu_effects_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
```

`mixer-proof.json` records final artifacts and results. Four new mixer tests bring
this isolated checkpoint to **42 passing workspace tests**, including target
selection, drum restoration/tank addition, binary32 cancellation order and the
predelay exclusion invariant. Native frozen-C/fresh-C0/Rust comparisons retain
all 23 fingerprints across **12,246,104 frames**, including all 19 genres, FX
transitions, direct engagement, impulse/null chains, output bounds and fixed USB.

The expanded worker matrix compares full frozen-C/fresh-C0/Rust PCM bytes for **48 cases**:
serial, deferred, pthread and busy/refused submissions; three tank types; both
retained work placements; both predelay endpoints; and maximum echo/size. That is
11,138,400 rendered frames per implementation. Synthetic abandoned-live-thread
retirement remains excluded.

The full WASM suite retains **40,058,180** compared Float32 values, plus
1,048,576 in the HTTP fallback, and passes framebuffer/save/restart checks. The
independent frozen R01 panel-effects comparison retains `fd659ab9` over 2,769,480
values, peak 0.453861. Factory and saved-v1 fingerprints remain `baeff971` and
`28722449`. The 24-export boundary and 32 MiB playback memory remain unchanged.
The Rust-enabled artifact is 2,462,323 bytes, **6,072 bytes larger** than the prior
isolated FX checkpoint; fresh C0 is 1,940,097 bytes. These are WASM sizes, not a
physical firmware map or an isolated MCU cost estimate.

No physical target was accessed or flashed. PI32 link/map and pool reserve,
soft-double symbol absence, interrupt stack, cycles/deadlines, worker exclusion
and retirement, and physical output qualification remain open. Native/WASM
parity does not establish those properties or new manual browser/device QA.
