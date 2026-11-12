# Browser audio buffers and retained views

The browser's stereo ring, sample conversion, bounded output drain and RGB565
background conversion now run through safe `fm1-app::web_audio`. The default
browser builds select `EN_RUST_WEB=1`; setting it to zero retains the C oracle.
The bridge's `web` feature is forbidden with `pi32v2`.

The payload remains 41,016 bytes: 4,096 stereo f32 frames, two 1,024-frame output
planes, two u32 positions and twelve background f32 values. No duplicate ring,
heap allocation, per-render sample packet or callback is introduced. Position
arithmetic wraps as unsigned 32-bit arithmetic; legacy overwrite on an abnormal
ring overrun is preserved. Normal rendering requests at most 1,024 frames and
produces blocks of 256, leaving at most 1,279 waiting frames. Short reads reject
without consuming or clearing output tails. The bridge releases each audio
borrow before advancing the Rust device clock, which can publish more audio.

## Retained output addresses

The browser keeps read-only typed views into WASM memory. Audio state, left
output, right output and background therefore have separate private
`UnsafeCell` roots. Every persistent raw output address comes directly from its
cell's `get()`. It is never derived from a temporary slice/reference that a
later exclusive update could invalidate. Drain borrows the three disjoint
state/output cells only for that call; background conversion borrows only its
own cell. No callback or reference survives a call.

All bridge calls are unsafe and require one serialized audio thread. C/JS view
reads happen between calls; they must not overlap Rust borrows or run from a
second worker. The private `Sync` implementation supplies that explicit FFI
contract, not concurrent access. An independent boundary review and the pinned
Miri strict-provenance retained-pointer test pass. Miri models the Rust pointer
accesses; it does not qualify hardware DMA or browser scheduling.

## Audio callback bindings

The worklet creates bounded output views for lengths zero through 1,024 at
startup and reuses them. Larger browser quanta drain in bounded chunks. The
HTTP fallback also reuses its input/output array wrappers. WASI's DataView is
cached. The WASM maximum equals its initial 32 MiB; attempted memory growth is
rejected, preserving cached views. Existing exports and messages remain intact.

The focused constructor check covers audio copies/WASI views with frame
publication disabled. Frame/store message construction, copies and browser
structured-clone transport remain binding work; this checkpoint does not claim
that the entire JavaScript processor allocates nothing. Platform state,
frame/store access and clock orchestration now use the shared Rust host model;
the C application ABI adapter remains for the controller migration. See
[HOST_PLATFORM.md](HOST_PLATFORM.md) for callback and pointer contracts.

## Reproduction

```sh
sh tests/rust_web_audio.sh /absolute/frozen-R01/source
sh tests/rust_emulator_parity.sh
node tests/host/emu_browser_test.mjs build/emu/amb.wasm
node tests/host/emu_audio_views_test.mjs build/emu-parity/reference.wasm build/emu-parity/rust.wasm
sh tests/rust_web_audio_miri.sh
```

The interpreter gate needs `miri` and `rust-src` for nightly-2026-08-28. Standard
production checks use Rust 1.94.1. Frozen C, fresh C0 and Rust compare 4,096
ring/output cases and all 65,536 background colors on native and WASM. The
[proof record](browser-audio-proof.json) retains hashes, full-instrument results
and the allocation-check scope. No physical device or live preview was changed.
