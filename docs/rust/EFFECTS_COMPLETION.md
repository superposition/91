# R12 effect settings and block traversal

This completes the remaining software effects scope on top of the
[original DSP checkpoint](EFFECTS.md), [outer mixer](MIXER.md) and
[worker protocol](WORKER_HANDOFF.md). `EN_RUST_EFFECTS` still selects the port;
the original C0 path remains available. Device link, memory and timing acceptance
remains separate. No physical device was accessed for this follow-up.

## Active call map

| Path | Rust behavior | Retained C seam |
| --- | --- | --- |
| Effect settings | Echo/reverb squared sends, size to feedback/decay, noise pump, chorus amount/shedding, duck decay, trim and output-level smoothing | Parameter validation/dispatch, immutable trim-table lookup, scalar capture/publication |
| Grid effects | Flex step/arm/quantization/held/one-shot transitions, repeat refresh and varispeed leave | Scheduler emits the same ordered callback opcodes; adapter captures the immutable 27×16 Flex bytes |
| Tank render | Ordered predelay samples through tank/shimmer, with one state capture and diagnostic increment per job | Select exactly two possible predelay read spans and the existing output spans |
| Performance/tone render | One block loop runs sampler then tone on each sample, with the original RNG and processing order | Capture/copy back scalar state once per block; supply existing ring/delays/mix spans |
| Predelay input | Echo/send/dry arithmetic and original scale/clamp/round quantization | Store already-quantized words and advance the existing cursor in sample order |
| Compressor/limiter/output | Existing Rust master DSP plus master/fixed-USB gains and final gap-then-hush integer fades | Render traversal, output-pointer selection and public meter publication |
| Resets | Original partial tank/echo scalar and payload resets | Establish storage addresses/lengths, choose when reset is legal, clear the shared predelay only with exclusive ownership |

Voice generation, DJ/snippet generation, general musical modulation, transport,
worker placement/join/retirement and application/storage/command dispatch are
their existing separate boundaries. UI spectrum analysis still reads the
performance ring through the C UI path; it must remain serialized with the
foreground render lease. C capture/publication and application dispatch remain
explicit #15 seams. This follow-up does not claim those application/UI ports.

The C FX adapters now contain scalar copies, bounded pointer/span selection,
storage writes and dispatch. Effect processing/configuration loops execute in
safe `no_std` Rust. The musical core keeps `forbid(unsafe_code)` and uses no
allocator, retained pointers or additional audio/voice bank. Flags, statistics
28..31, fixed payloads, voice limits, 24 WASM exports and 32 MiB emulator memory
remain unchanged.

## Spans, order and partial reset compatibility

The tank adapter forms immutable references only to the exact read span before
predelay wrap and its exact wrapped remainder. Its mutable tank borrow starts
after the 1,579-word shared predelay. The foreground writer stays in its existing
disjoint span; no whole predelay reference is created. Both split spans are
processed inside one bridge call and one diagnostic increment. Tests retain both
spacing endpoints, both placements and actual pthread execution. Unknown worker
ownership still quarantines every leased span; timeout does not permit reset.

The existing `rsend` working slots carry already-quantized i16 predelay words as
f32 after the echo phase consumes their old values. Every i16 is exactly
representable in f32. C only casts/stores these words, in original order. This
removes per-sample C quantization without an extra payload, whole-ring borrow or
per-sample foreign call. The earlier spacing proof remains applicable.

Sampler/tone configuration is stable within a chunk: effect settings and knobs
are queued and drained by the foreground before rendering. One captured packet
therefore supplies the same values as the old per-sample captures. The new Rust
loop preserves sampler→tone interleaving on each frame. Tone processing makes no
RNG draw, so the shared foreground RNG draw order also stays unchanged. DJ voices
remain before sidechain and do not borrow the performance ring.

The gap pass still truncates main/USB integers before the separate hush pass.
Combining the two gains would change output bytes. Zero-frame render calls still
smooth trim and update the hush publication. Fixed USB keeps its original unity
gain and independent clipping behavior.

Reset behavior is deliberately partial. Tank reset clears tank/predelay storage,
line cursors, bandwidth/damping/DC state and the sine/cosine pair. It retains the
shimmer payload/write position/phase, gated envelope/gain/hold and shimmer
feedback. Echo initialization clears tape/plex payloads and cursors, resets head
position/target/speed and current length, and retains its existing filter/history
scalars and head pan/lean/timer. Normal default parameter application follows
that helper, including its original echo-length update. The independent oracle seeds nonzero state and
checks these retained values rather than assuming a complete reset.

## Scalar and synchronization cost

| Adapter | Previous scalar capture/config/copyback | Current real render |
| --- | --- | --- |
| Sampler + tone | 1,092 bytes per sample, excluding RNG word | Same 1,092 bytes once per block |
| Tank | 216 bytes per sample | Same 216 bytes once per job, including predelay wrap |
| Sampler/tone/tank diagnostics | Three atomic increments per sample | Two atomic increments per block/job, preserving sample totals |
| Echo, chorus, compressor | Already processed in blocks | Existing batching retained |

These are source packet/RMW counts, not compiler traffic, measured stack or MCU
cycles. Combined sampler/tone scalar packets total 724 bytes before copyback;
the old separate adapters used 252-byte and 472-byte packets with separate
lifetimes. Keeping both live adds a bounded 252 bytes to that source-level
maximum; actual linked stack use remains unmeasured. No audio array is placed
on the Rust stack. The vinyl config adds one
raw f32 field (40→44 bytes) so Rust combines fade and level. Raw scatter settings
and bus mix replace same-size normalized fields; their packets remain 116 and
40 bytes. The generated 32-byte grid packet has C/Rust size/offset assertions;
the immutable Flex extent is asserted as 27×16.

## Reproduction and evidence

Use Rust 1.94.1 and the immutable 440-source R01 checkout:

```sh
export RUSTUP_TOOLCHAIN=1.94.1
cargo test --workspace --locked
cargo test -p fm1-rt --features pi32v2 --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --release --no-default-features --locked
python3 tools/effects_abi.py --check
sh tests/rust_effects_completion.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source extra-genres
sh tests/rust_effects_workers.sh ../baseline-2026-11-11/source
sh tests/rust_fm6_workers.sh ../baseline-2026-11-11/source
sh tests/rust_worker_handoff.sh ../baseline-2026-11-11/source
sh tests/rust_melodic_parity.sh ../baseline-2026-11-11/source
sh tests/rust_handoff.sh ../baseline-2026-11-11/source
sh tests/rust_emulator_parity.sh
node tests/host/emu_worker_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_effects_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_fm6_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_browser_test.mjs build/emu-parity/rust.wasm
```

The focused frozen-C/fresh-C0/Rust oracle compares 1,276 settings/tempo/trim/
zero-frame rows (`27723a28`), 42,363 grid transitions (`98fe6947`), 94,452 actual
gain/gap/hush/USB frames with bounds canaries (`62c27989`) and 64 nonzero partial
reset cases (`fd353cc2`). The full native FX, worker, melodic and WASM suites
and their exact final artifact are recorded in `effects-completion-proof.json`.
An additional 505 chorus cases exercise every amount 0..100 and shed levels 0..4,
including the shed=2 threshold: 40,699 frames, fingerprint `1ea84c3a`.

The combined run passes 99 workspace tests and 14 runtime tests through the
opaque atomic ABI. Full render checks match 23 FX cases (12,246,104 frames),
48 FX worker configurations (11,138,400), 32 FM worker configurations (7,425,600),
36 handoff/retirement cases (8,353,800) and 43 melodic scenarios (80,844,411 frames
and 209,246 MIDI messages). USB busy/readback/save/reboot retains `eff97ff8`.

The expanded WASM suite compares 47,059,496 Float32 values with fresh C0, plus
1,048,576 HTTP fallback values, and retains every prior PCM/framebuffer/save/
restart fingerprint. Independent frozen-C worker/FM6/effects checks use the same
final artifact. Browser lifecycle checks are Node mocks; no manual browser,
mobile, audio-device or MIDI QA is claimed here.

The final Rust WASM is 2,406,097 bytes, SHA-256
`9ec3e02b31d3215b4de8eb628910ea31cc2b223d32c06128804a6bb3e00c2f85`.
Fresh C0 is 1,941,411 bytes. The Rust artifact grows 8,892 bytes relative to the
prior worker checkpoint; batching is not a code-size saving claim. Both current
WASM artifacts reproduce byte-for-byte on a fresh rebuild.

Physical integrated link/map, pool reserve, soft-double symbol absence, interrupt
stack, stop/cache/IRQ visibility, cycles/deadlines and output qualification remain
root/platform acceptance. Host equivalence and source batching counts do not
establish those device properties.
