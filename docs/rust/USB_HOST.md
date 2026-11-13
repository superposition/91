# USB-MIDI host selection and RX policy (R18 / #19)

This checkpoint follows [retained endpoint lifetimes](USB_ENDPOINTS.md). The
real host's `uh_parse_midi`, `uh_midi_ok` and active `uh_poll` RX packet traversal
now use safe `no_std` Rust behind `EN_RUST_USB`. The original C0 implementations
remain selectable. This is a software port; physical USB1/stop-provider
qualification remains open and the known working firmware is untouched.

`fm1-hal::usb_host` selects the first MIDI 1.0 streaming alternate with a usable
full-speed endpoint, without combining interfaces or alternates. Descriptor
lengths, truncation, endpoint address/type/interval/packet-size checks, output-only
controllers and interrupt endpoint rounding retain C0 behavior. It reads the
existing CPU-owned configuration copy, bounded to its existing 768-byte limit;
this input is not a borrowed DMA buffer. Enumeration has device polling and host
RX polling disabled while this scoped immutable configuration borrow exists.

The 16-byte result preserves every original C field/offset and has two explicit
initialized padding bytes. The bridge writes the complete value through a raw
output pointer; it never creates `&mut` over an uninitialized C result. Input and
output overlap, alignment, null and size checks precede any input reference.
Neither a configuration pointer nor a result reference survives the call.

Host packet policy drops cable numbers, rejects SysEx/editor commands and admits
the same channel/CIN/status pairs and clock/start/continue/stop bytes. Existing
compatibility quirks remain: realtime trailing bytes and the unused third byte of
two-byte channel messages are preserved, rather than silently sanitized.

Active RX crosses the bridge **once per <=64-byte packet**. Safe Rust retains the
existing conservative sixteen-free-slot rule, ignores a partial final word,
filters each message, writes its bounded queue slot, publishes, then advances the
producer index with wrapping arithmetic. There is no per-message FFI call.
The C wrapper publishes the returned accepted-event total after the scoped call;
actual callbacks are only leaf memory barriers, and foreground diagnostics cannot
observe the middle of TIMER5. End-of-packet counts and ordering remain exact.

The scoped RX descriptor contains five raw roots: retained RX, existing 64-word
MIDI input queue, write/read indices and a leaf publication binding. No Rust
reference is formed to the DMA payload, queue slots or indices. The producer
role stays serialized with USB/TRS, and the independent consumer can retire
published slots without a whole-ring mutable borrow. Known-ready CSR/COUNT,
cache synchronization, packet retention on insufficient room and release/next
request MMIO stay in their existing C order. Unknown completion still follows
the preceding endpoint quarantine/confirmed-stop contract.

## Fixed storage and measured C object cost

There is no allocator, new writable global, payload, audio buffer, copied RX
packet or expanded runtime cache. The endpoint result stays 16 bytes (alignment
2, input MPS at 8 and output MPS at 14). The RX descriptor is 20 bytes on PI32v2 /
40 on native64. In the actual vendor `-Os` object, the compiler hoists the
source-local const descriptor into **20 bytes of immutable
`.rodata.uh_poll.context`**; it is not necessarily caller-stack allocation and is
not claimed free. This descriptor holds existing addresses only.

With USB1/HAL0 C objects, compared to `2855811`, `usb_poll` remains 60 B,
`fm1_timer5_irq` 60 B and `uh_ctl` 40 B including pushed registers. Main and engine
compile for USB1/HAL0 and HAL1 with exact C ABI/layout assertions. These are C
object facts. New Rust entries still need adopted-mrustc emission, whole-link /
LTO and complete stack/timing qualification; no generated-Rust or physical cost
is inferred from the C frame sizes or test timings.

## Independent checks

```sh
sh tests/rust_usb_host.sh ../baseline-2026-11-11/source
```

Pinned Rust 1.94.1 uses the independently verified 440-file frozen source and fresh
C0. The fixture calls the real header parser and real driver `uh_poll` with its
retained-DMA/register double. It covers every descriptor byte mutation, all
truncations, every endpoint address/type and packet-size/interval boundary,
long prefixes, malformed configurations, all 65,536 cable/CIN/status combinations
with four data-boundary patterns, and every 0..64 packet length/free-ring count
at ordinary and wrapping indices with eight accepted/rejected/error patterns.
All serialized state, every MIDI queue word and retained source byte, plus the
register/payload digest agree across frozen C, fresh C0, Rust native and both
WASM consumers: **552,622 cases / 38,947,344 B**, SHA-256
`e0a13b6211c51c4670552d2665b7aadafe2f42a75e044645494989e2cd91c534`.
The ten original named host checks also pass byte-identically. The endpoint
retention/stop/reset/abort correction matrix also reruns in its own output
folder: frozen C0 and corrected Rust keep the exact hashes from `2855811`,
including all retained-source refusal/recovery assertions. Checks remain
active under optimized `-DNDEBUG`; test buffering is fixture-only.

All 175 workspace tests pass, along with fmt, warnings-denied Clippy and
no_std/WASM checks. Strict-provenance Miri passes all five focused tests,
including full raw initialization of the C result, retained RX/queue roots
between calls, admission, wrapping/publication order and compatibility quirks.
Sources, explicit command exit codes, logs, native/WASM and vendor object hashes
are in [usb-host-proof.json](usb-host-proof.json).

## Remaining #19 boundaries

Host TX retained ACK/FIFO retry policy, enumeration/control/hub search and
attach/loss/restart traversal still have active C bodies. CDC services and UAC
stream/ring/window/resampling/packet policy remain C. Device poll/reset/retry
orchestration and thin physical MMIO/cache/DMA bindings also remain C. The
already ported descriptor/EP0, device TX, SysEx/RX, MIDI queues and TRS paths stay
active Rust seams. The endpoint unknown/reset/abort correction stays intact.

Actual positive stop/retirement providers, combined device/host/control/UAC/TRS
traffic, cache/DMA/electrical behavior, fixed/master recording and deadlines
remain hardware acceptance gates. No device, flash, live server, remote issue,
emitter or main application dispatch was changed.
