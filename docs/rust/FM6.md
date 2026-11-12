# Six-operator FM engine and fixed voice pool

R11 / [issue #12](https://github.com/superposition/91/issues/12) now uses the
Rust FM6 engine in actual native/WASM playback. This covers the operator and
envelope kernels, all 32 algorithm plans, Modern/Mark I/OPL models, integer and
float Mark I paths, fixed allocation, two Songbook roles, program selection,
profiles, user-bank decisions and the FM FIFO/output arithmetic. Original C0
implementations remain behind `EN_RUST_FM6`; physical firmware does not enable
the flag. Browser builders permit `EN_RUST_FM6=0` independently of other ports.

## Processing and representation

`fm1-engine::fm6` is safe, allocator-free `no_std` code under the crate's
`forbid(unsafe_code)`. Modules separate table initialization, scaling, amplitude
and pitch envelopes, LFOs, operators, routing plans, allocation, programs and
the 64-frame render queue. `fm1-jam-bridge::fm6` is the scoped foreign-memory
adapter; `rust_fm6.h` mirrors its numeric ABI. It retains no pointer or callback.

The existing six 408-byte slots remain in C storage (2,448 bytes). Each embeds
a 388-byte voice: six amplitude envelopes, operator phases/frequencies/gains,
feedback, pitch envelope and plan/cache flags. Amplitude envelopes are 20 bytes,
pitch envelopes 24 bytes and each LFO 20 bytes. Byte flags are not foreign Rust
enums or booleans. Size/offset assertions and field-wise independent traces
check the native/WASM layout without reading struct padding.

Legacy allocation retains the six/four/two load-shedding caps and the 46 program
caps. Songbook has five keyboard slots (0,1,2,3,5), one protected bass slot (4)
and separate keyboard/bass LFOs. Bass renders last, leaving its output in the
existing operator sum while keyboard output accumulates in the queue. The
reverse swell uses the separate analog pad voices; it is not a third FM role.

Allocation preserves strict numerical unsigned age ordering, first-key LFO
restart, free-slot key-sync phase reset, stolen-slot phase/gain/feedback
continuity, gate expiry, quiet retirement and bounded release fades. Keys fade
over 35 blocks in Songbook; legacy keys and bass use 103 blocks. Engine changes
retain the original gain-unit reset. Reset and program changes deliberately
preserve their different age, bass-LFO and scratch/cache behavior. An inactive
program still zeros the keyboard queue without clearing the old operator sum.

There is one known compatibility quirk: a Songbook bass candidate with age
`UINT32_MAX` does not satisfy `age < oldest`, leaving the original default
candidate at slot 0. The next bass attack therefore steals slot 0 while slot 4
remains active. Immutable R01, fresh C0 and Rust reproduce this exact endpoint.
Changing allocation to enforce protection across that endpoint is a separate
allocator-fix follow-up, with an explicit audible/behavioral acceptance change.

Arithmetic retains binary32 order without FMA, Q24 phase wrapping, exact
frequency partial products, signed envelope shifts, gain ramps, silent phase
advance, feedback loops for algorithms 4/6 and the algorithm-32 feedback scale.
One mutable scratch view owns both buses; tagged bus indices perform a scalar
read before a write when modulation input and output are the same bus. No pair
of aliased immutable/mutable Rust slices is constructed.

## Tables, programs and memory

`tools/import_fm6_tables.py` imports exact independently exported C values,
including float bits. It supplies 8 packed factory voices, 24 canonical ROM
voices, the unchanged 30-program mapping, 30 DY voices, 46 caps/gains, names,
algorithm/scaling/frequency/LFO tables and compressed model ROMs. Stale original
comments describing 16 ROM voices do not set the actual capacity. The generated
source declares 21,281 bytes of primitive immutable values, including historical
unused INIT/portamento data; this is a source payload count, not linked flash.
ROM presets use the 155-byte canonical representation produced by frozen C.
Packed factory/DY/user loading and profiles reuse `fm1-formats::voice` from R05;
C callers keep their existing extra byte-155 NUL terminator.

RAM capacities and placement are unchanged:

| Existing region | Bytes |
| --- | ---: |
| Initialized sine/Mark I/OPL lookup buffers | 11,780 |
| Float operator buses and sum | 768 |
| Integer operator buses and sum | 768 |
| 64-frame keyboard queue | 256 |
| 512-frame FM FIFO | 2,048 |
| 16 packed user slots | 2,048 |
| Shared 16-upload-slot / 512-frame bass-FIFO union | 2,048 |
| Two canonical C patch buffers, including terminators | 312 |
| User display names | 256 |

There is no new persistent voice bank, audio buffer, allocator, expanded cache
or large Rust stack array. The only new persistent diagnostic storage is eight
32-bit counters (32 bytes), exposed through existing `web_rust_stat` indices
48..55: generated blocks, accepted notes, steals, gate releases, program/profile
loads, user commits, generated FIFO frames and resets. They add no WASM export.

The live FM job captures configuration, table/scratch descriptors and the
24-byte AMS cache once per job. Together with its 12-pointer render descriptor,
these source packets occupy 132 bytes on a 32-bit ABI. The cache is copied back
once per job, rather than once per sample. Foreground output uses a 20-byte gain
packet plus four pointers per read span. These counts are not measured stack
high-water marks or memory traffic. Ordinary 256-frame jobs use one Rust call;
a wrap uses at most two. Samples go directly into the existing FIFO and four
existing output arrays, with the original number of audio writes.

The adapter borrows only each selected contiguous FIFO span, never the entire
shared 512-frame ring. Producer and consumer spans are 256 frames apart, each
at most 256 frames. The C owner still joins the previous worker before changing
FM state or programs. The bass union is not borrowed at all outside Songbook;
legacy upload staging is refused while that union serves as bass audio. User
validation/masking/count/pending/name decisions use Rust, while C retains the
existing command publication and serialized staging lifecycle. This does not
invent a new concurrent transfer protocol.

## Remaining C call map

The active C seam retains allocation/placement of pool/BSS arrays and scalars,
parameter/role/config capture, command publication, MIDI/phrase trigger call
order, Songbook dirty flags, explicit reset sequencing, boot/storage/UI
accessors and display-pointer assignment. `fm_jam_load` still resets the bass
LFO/FIFO and staging pending mask in the existing order. Worker submit/join,
fences, stage publication, fallback/retirement and render traversal remain C
plumbing. Stop/Start remains coordinated by the existing application/scheduler
path; gate/tail processing and FM state reset execute Rust when invoked there.

The inactive original `fm6_core.inc` kernels, ROM conversion helper and C
selection/allocator bodies remain for `EN_RUST_FM6=0` and the frozen oracles.
`fm_next` remains a fallback/test wrapper; the live Rust FM job uses the full
span adapter. C still calculates the surrounding track/volume control packet
and maps returned peaks to the existing meters. Analog pad/swell, percussion,
plain pluck, bass/acid/noise ports belong to R10; outer mixing/effects belong
to R12. This change does not claim those separate ports.

## Qualification and reproduction

Use Rust **1.94.1**, Node **22.14.0**, the existing clang WASM wrapper and the
immutable R01 source/archive. Each independent native script checks all 440
source hashes before compiling its reference. Reproduction commands:

```sh
export RUSTUP_TOOLCHAIN=1.94.1
cargo fmt --all --check
cargo test --workspace --locked
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo check --workspace --release --no-default-features --locked
sh tests/rust_fm6_tables.sh ../baseline-2026-11-11/source
sh tests/rust_fm6_parity.sh ../baseline-2026-11-11/source
sh tests/rust_fm6_workers.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source
sh tests/rust_effects_parity.sh ../baseline-2026-11-11/source extra-genres
sh tests/rust_effects_workers.sh ../baseline-2026-11-11/source
sh tests/rust_emulator_parity.sh
node tests/host/emu_fm6_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
node tests/host/emu_effects_test.mjs ../baseline-2026-11-11/source/build/emu/amb.wasm build/emu-parity/rust.wasm --audio-only
sh tests/rust_fm6_benchmark.sh ../baseline-2026-11-11/source
```

`fm6-proof.json` records checks, fingerprints, sizes and host costs. The nine FM
tests bring this base to 78 workspace tests. Independent components cover
3,456 per-operator bus/add/feedback/path/partial-block cases, 48 two/three-op
feedback loops, 900 amplitude envelopes, 300 pitch envelopes and 1,600 LFO
wave/rate/delay-wrap cases: 6,304 vectors. All initialized lookup values also
match frozen C bits. Full native FM traces compare every relevant state field,
scratch value and PCM byte through all 32 algorithms/three models, both Mark I
paths, all 60 factory/DY programs, all 16 user slots, protected roles/profiles,
retrigger/release/reset and irregular FIFO/counter wraps. Eight scenarios cover
2,225,846 frames; the separate full-engine suite retains 23 fingerprints over
12,246,104 frames, including all 19 genres and fixed USB/master output.

The original 48 worker cases still match, and FM qualification adds 32 cases
covering legacy and actual v2 keyboard/bass rendering, both Mark I paths, both
work placements, serial/deferred/pthread/busy submissions and irregular chunks.
Synthetic abandoned-live-thread tests remain excluded. This qualifies the host
completion model, not a physical worker exclusion/retirement guarantee.

Full WASM regression retains the original 41,028,380 compared Float32 values,
framebuffers, saves/restarts and 1,048,576-value HTTP fallback. Four additional
loaded-user algorithm tests compare 4,752,216 values against fresh C0 and R01.
They use a saved legacy/off session and seed 0: default Songbook boot refuses
queued legacy voice commits while its union carries bass audio, and random
piece selection would not reliably select the stored user program. Actual user
commit/program/frame counters and four distinct PCM hashes prove activation.
The 24-export ABI and 32 MiB memory remain unchanged. Factory/saved-v1 hashes
remain `baeff971`/`28722449`; the independent panel hash remains `fd659ab9`.

The final Rust-enabled WASM is 2,425,930 bytes; all-Rust-except-FM fallback is
2,453,012 bytes (27,082 bytes larger), and full C0 is 1,940,655 bytes. The
independent FM fallback also reproduces all four loaded-voice outputs. These
are host/WASM artifact sizes, not a PI32 firmware map or target flash budget.

Host benchmark results are recorded separately in the proof. They include
real 256-frame job, queue/FIFO/output adapters, gates and periodic six-voice or
five-key-plus-one-bass retriggers, compiled release with contraction disabled.
Three trials per model/path use one host CPU affinity. They are host costs,
including transitional FFI overhead, with no interpretation as PI32 timing.

| Host workload | Frozen C ns/frame | Rust ns/frame | Rust / C |
| --- | ---: | ---: | ---: |
| Modern | 58.69 | 97.89 | 1.668 |
| Mark I integer | 72.89 | 74.21 | 1.018 |
| OPL | 81.67 | 116.72 | 1.429 |
| Mark I float | 78.84 | 104.90 | 1.330 |
| Songbook five keys + bass | 132.76 | 129.39 | 0.975 |

Modern, OPL and float Mark I have material host cost increases in this port.
The proof preserves all raw trials and fresh-C0 measurements. Model dispatch,
bounds-check and code-generation costs are candidates for measured follow-up;
this checkpoint does not claim a speed improvement or target budget clearance.

No physical target was accessed or flashed. Remaining R11 acceptance is PI32
link/map and pool reserve, soft-double/library-symbol audit, interrupt/worker
stack high-water marks, worst-case cycles/deadlines and actual worker lifetime
exclusion/retirement, plus device output/listening qualification. Native/WASM
parity does not establish these, or new manual desktop/mobile/audio/MIDI QA.
